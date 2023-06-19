use crate::{reflector::ObjectRef, watcher::Error};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{ready, Stream};
use kube_client::Resource;
use pin_project::pin_project;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

fn hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

/// A predicate is a hasher of Kubernetes objects stream filtering
pub trait Predicate<K> {
    /// A predicate only needs to implement optional hashing when keys exist
    fn hash_property(&self, obj: &K) -> Option<u64>;

    /// Returns a `Predicate` that falls back to an alternate property if the first does not exist
    ///
    /// # Usage
    ///
    /// ```
    /// # use k8s_openapi::api::core::v1::Pod;
    /// use kube::runtime::{predicates, Predicate};
    /// # fn blah<K>(a: impl Predicate<K>) {}
    /// let pred = predicates::generation.fallback(predicates::resource_version);
    /// blah::<Pod>(pred);
    /// ```
    fn fallback<F: Predicate<K>>(self, f: F) -> Fallback<Self, F>
    where
        Self: Sized,
    {
        Fallback(self, f)
    }

    /// Returns a `Predicate` that combines all available hashes
    ///
    /// # Usage
    ///
    /// ```
    /// # use k8s_openapi::api::core::v1::Pod;
    /// use kube::runtime::{predicates, Predicate};
    /// # fn blah<K>(a: impl Predicate<K>) {}
    /// let pred = predicates::labels.combine(predicates::annotations);
    /// blah::<Pod>(pred);
    /// ```
    fn combine<F: Predicate<K>>(self, f: F) -> Combine<Self, F>
    where
        Self: Sized,
    {
        Combine(self, f)
    }
}

impl<K, F: Fn(&K) -> Option<u64>> Predicate<K> for F {
    fn hash_property(&self, obj: &K) -> Option<u64> {
        (self)(obj)
    }
}

/// See [`Predicate::fallback`]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Fallback<A, B>(pub(super) A, pub(super) B);
impl<A, B, K> Predicate<K> for Fallback<A, B>
where
    A: Predicate<K>,
    B: Predicate<K>,
{
    fn hash_property(&self, obj: &K) -> Option<u64> {
        self.0.hash_property(obj).or_else(|| self.1.hash_property(obj))
    }
}
/// See [`Predicate::combine`]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Combine<A, B>(pub(super) A, pub(super) B);
impl<A, B, K> Predicate<K> for Combine<A, B>
where
    A: Predicate<K>,
    B: Predicate<K>,
{
    fn hash_property(&self, obj: &K) -> Option<u64> {
        match (self.0.hash_property(obj), self.1.hash_property(obj)) {
            // pass on both missing properties so people can chain .fallback
            (None, None) => None,
            // but any other combination of properties are hashed together
            (a, b) => Some(hash(&(a, b))),
        }
    }
}

#[allow(clippy::pedantic)]
#[pin_project]
/// Stream returned by the [`predicate_filter`](super::WatchStreamExt::predicate_filter) method.
#[must_use = "streams do nothing unless polled"]
pub struct PredicateFilter<St, K: Resource, P: Predicate<K>> {
    #[pin]
    stream: St,
    predicate: P,
    cache: HashMap<ObjectRef<K>, u64>,
}
impl<St, K, P> PredicateFilter<St, K, P>
where
    St: Stream<Item = Result<K, Error>>,
    K: Resource,
    P: Predicate<K>,
{
    pub(super) fn new(stream: St, predicate: P) -> Self {
        Self {
            stream,
            predicate,
            cache: HashMap::new(),
        }
    }
}
impl<St, K, P> Stream for PredicateFilter<St, K, P>
where
    St: Stream<Item = Result<K, Error>>,
    K: Resource,
    K::DynamicType: Default + Eq + Hash,
    P: Predicate<K>,
{
    type Item = Result<K, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        Poll::Ready(loop {
            break match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(obj)) => {
                    if let Some(val) = me.predicate.hash_property(&obj) {
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

/// Predicate functions for [`WatchStreamExt::predicate_filter`](crate::WatchStreamExt::predicate_filter)
///
/// These functions just return a hash of commonly compared values,
/// to help decide whether to pass a watch event along or not.
///
/// Functional rewrite of the [controller-runtime/predicate module](https://github.com/kubernetes-sigs/controller-runtime/blob/main/pkg/predicate/predicate.go).
pub mod predicates {
    use super::hash;
    use kube_client::{Resource, ResourceExt};

    /// Hash the generation of a Resource K
    pub fn generation<K: Resource>(obj: &K) -> Option<u64> {
        obj.meta().generation.map(|g| hash(&g))
    }

    /// Hash the resource version of a Resource K
    pub fn resource_version<K: Resource>(obj: &K) -> Option<u64> {
        obj.meta().resource_version.as_ref().map(hash)
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
