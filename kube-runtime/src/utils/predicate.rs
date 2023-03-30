use crate::{reflector::ObjectRef, watcher::Error};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{ready, Stream, TryStream};
use kube_client::Resource;
use pin_project::pin_project;
use std::{collections::HashMap, hash::Hash};

#[allow(clippy::pedantic)]
#[pin_project]
/// Stream returned by the [`predicate_filter`](super::WatchStreamExt::predicate_filter) method.
#[must_use = "streams do nothing unless polled"]
#[allow(clippy::type_complexity)]
pub struct PredicateFilter<St, K: Resource, Func> {
    #[pin]
    stream: St,
    predicate: Box<dyn Func>,
    cache: HashMap<ObjectRef<K>, u64>,
}
impl<St, K, F> PredicateFilter<St, K, F> {
    pub(super) fn new(stream: St, predicate: F) -> Self
    where
        St: TryStream<Ok = K>,
        K: Resource,
        F: Fn(&K) -> Option<u64> + 'static + Send,
    {
        Self {
            stream,
            predicate: Box::new(predicate),
            cache: HashMap::new(),
        }
    }
}
impl<St, K, F> Stream for PredicateFilter<St, K, F>
where
    St: Stream<Item = Result<K, Error>>,
    K: Resource,
    K::DynamicType: Default + Eq + Hash,
    F: Fn(&K) -> Option<u64> + 'static + Send,
{
    type Item = Result<K, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        Poll::Ready(loop {
            break match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(obj)) => {
                    if let Some(val) = (me.predicate)(&obj) {
                        let key = ObjectRef::from_obj(&obj);
                        let changed = if let Some(old) = me.cache.get(&key) {
                            *old != val
                        } else {
                            true
                        };
                        if let Some(old) = me.cache.get_mut(&key) {
                            *old = val;
                        } else {
                            me.cache.insert(key, val);
                        }
                        if changed {
                            Some(Ok(obj))
                        } else {
                            continue;
                        }
                    } else {
                        // if we can't evaluate predicate, always emit K
                        Some(Ok(obj))
                    }
                }
                Some(Err(err)) => Some(Err(err)),
                None => return Poll::Ready(None),
            };
        })
    }
}

pub mod predicates {
    use kube_client::{Resource, ResourceExt};
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    // See: https://github.com/kubernetes-sigs/controller-runtime/blob/v0.12.0/pkg/predicate/predicate.go

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash the generation of a Resource K
    pub fn generation<K: Resource>(obj: &K) -> Option<u64> {
        obj.meta().generation.map(|g| hash(&g))
    }

    /// Hash the labels of a Resource K
    pub fn labels<K: Resource>(obj: &K) -> Option<u64> {
        Some(hash(obj.labels()))
    }

    /// Hash the annotations of a Resource K
    pub fn annotations<K: Resource>(obj: &K) -> Option<u64> {
        Some(hash(obj.annotations()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::task::Poll;

    use super::{predicates, Error, PredicateFilter};
    use futures::{pin_mut, poll, stream, FutureExt, StreamExt};
    use kube_client::Resource;
    use serde_json::json;

    #[tokio::test]
    async fn predicate_filtering_hides_equal_predicate_values() {
        use k8s_openapi::api::core::v1::Pod;
        let mkobj = |gen: i32| {
            let p: Pod = serde_json::from_value(json!({
                "apiVersion": "v1",
                "kind": "Pod",
                "metadata": {
                    "name": "blog",
                    "generation": Some(gen),
                },
                "spec": {
                    "containers": [{
                      "name": "blog",
                      "image": "clux/blog:0.1.0"
                    }],
                }
            }))
            .unwrap();
            p
        };
        let data = stream::iter([
            Ok(mkobj(1)),
            Err(Error::TooManyObjects),
            Ok(mkobj(1)),
            Ok(mkobj(2)),
        ]);
        let rx = PredicateFilter::new(data, predicates::generation);
        pin_mut!(rx);

        // mkobj(1) passed through
        let first = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(first.meta().generation, Some(1));

        // Error passed through
        assert!(matches!(
            poll!(rx.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        // (no repeat mkobj(1) - same generation)
        // mkobj(2) next
        let second = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(second.meta().generation, Some(2));
        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }
}
