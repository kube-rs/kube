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
pub struct PredicateFilter<St, K: Resource, V: PartialEq> {
    #[pin]
    stream: St,
    predicate: Box<dyn (Fn(&K) -> Option<V>)>,
    // TODO: HashMap should only store a Hash of V
    cache: HashMap<ObjectRef<K>, V>,
}
impl<St: TryStream<Ok = K>, K: Resource, V: PartialEq> PredicateFilter<St, K, V> {
    pub(super) fn new(stream: St, predicate: impl Fn(&K) -> Option<V> + 'static) -> Self {
        Self {
            stream,
            predicate: Box::new(predicate),
            cache: HashMap::new(),
        }
    }
}
impl<St, K, V> Stream for PredicateFilter<St, K, V>
where
    St: Stream<Item = Result<K, Error>>,
    V: PartialEq,
    K: Resource,
    K::DynamicType: Default + Eq + Hash,
{
    type Item = Result<K, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        Poll::Ready(loop {
            break match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(obj)) => {
                    if let Some(val) = (me.predicate)(&obj) {
                        // TODO: hash value here
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
    use kube_client::Resource;
    use std::collections::BTreeMap;
    // TODO: import from https://github.com/kubernetes-sigs/controller-runtime/blob/v0.12.0/pkg/predicate/predicate.go

    /// Compute the generation of a Resource K
    pub fn generation<K: Resource>(x: &K) -> Option<i64> {
        x.meta().generation
    }

    /// Compute the labels of a Resource K
    pub fn labels<K: Resource>(x: &K) -> Option<BTreeMap<String, String>> {
        x.meta().labels.clone()
    }
}
