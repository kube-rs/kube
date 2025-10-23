use crate::watcher::Error;
use core::{
    pin::Pin,
    task::{ready, Context, Poll},
};
use futures::Stream;
use kube_client::{api::ObjectMeta, Resource};
use pin_project::pin_project;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    marker::PhantomData,
    time::{Duration, Instant},
};

fn hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

/// Private cache key that includes UID in equality for predicate filtering
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PredicateCacheKey {
    name: String,
    namespace: Option<String>,
    uid: Option<String>,
}

impl From<&ObjectMeta> for PredicateCacheKey {
    fn from(meta: &ObjectMeta) -> Self {
        Self {
            name: meta.name.clone().unwrap_or_default(),
            namespace: meta.namespace.clone(),
            uid: meta.uid.clone(),
        }
    }
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

/// Configuration for predicate filtering with cache TTL
#[derive(Debug, Clone)]
pub struct Config {
    /// Time-to-live for cache entries
    ///
    /// Entries older than this duration will be evicted from the cache.
    /// Default is 1 hour.
    pub ttl: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Default to 1 hour TTL - long enough to avoid unnecessary reconciles
            // but short enough to prevent unbounded memory growth
            ttl: Duration::from_secs(3600),
        }
    }
}

/// Cache entry storing predicate hash and last access time
#[derive(Debug, Clone)]
struct CacheEntry {
    hash: u64,
    last_seen: Instant,
}

#[allow(clippy::pedantic)]
#[pin_project]
/// Stream returned by the [`predicate_filter`](super::WatchStreamExt::predicate_filter) method.
#[must_use = "streams do nothing unless polled"]
pub struct PredicateFilter<St, K: Resource, P: Predicate<K>> {
    #[pin]
    stream: St,
    predicate: P,
    cache: HashMap<PredicateCacheKey, CacheEntry>,
    config: Config,
    // K: Resource necessary to get .meta() of an object during polling
    _phantom: PhantomData<K>,
}
impl<St, K, P> PredicateFilter<St, K, P>
where
    St: Stream<Item = Result<K, Error>>,
    K: Resource,
    P: Predicate<K>,
{
    pub(super) fn new(stream: St, predicate: P, config: Config) -> Self {
        Self {
            stream,
            predicate,
            cache: HashMap::new(),
            config,
            _phantom: PhantomData,
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

        // Evict expired entries before processing new events
        let now = Instant::now();
        let ttl = me.config.ttl;
        me.cache
            .retain(|_, entry| now.duration_since(entry.last_seen) < ttl);

        Poll::Ready(loop {
            break match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(obj)) => {
                    if let Some(val) = me.predicate.hash_property(&obj) {
                        let key = PredicateCacheKey::from(obj.meta());
                        let now = Instant::now();

                        // Check if the predicate value changed or entry doesn't exist
                        let changed = me.cache.get(&key).map(|entry| entry.hash) != Some(val);

                        // Update cache with new hash and timestamp
                        me.cache.insert(key, CacheEntry {
                            hash: val,
                            last_seen: now,
                        });

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

    /// Hash the finalizers of a Resource K
    pub fn finalizers<K: Resource>(obj: &K) -> Option<u64> {
        Some(hash(obj.finalizers()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{pin::pin, task::Poll};

    use super::{predicates, Config, Error, PredicateFilter};
    use futures::{poll, stream, FutureExt, StreamExt};
    use kube_client::Resource;
    use serde_json::json;

    #[tokio::test]
    async fn predicate_filtering_hides_equal_predicate_values() {
        use k8s_openapi::api::core::v1::Pod;
        let mkobj = |g: i32| {
            let p: Pod = serde_json::from_value(json!({
                "apiVersion": "v1",
                "kind": "Pod",
                "metadata": {
                    "name": "blog",
                    "generation": Some(g),
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
            Err(Error::NoResourceVersion),
            Ok(mkobj(1)),
            Ok(mkobj(2)),
        ]);
        let mut rx = pin!(PredicateFilter::new(
            data,
            predicates::generation,
            Config::default()
        ));

        // mkobj(1) passed through
        let first = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(first.meta().generation, Some(1));

        // Error passed through
        assert!(matches!(
            poll!(rx.next()),
            Poll::Ready(Some(Err(Error::NoResourceVersion)))
        ));
        // (no repeat mkobj(1) - same generation)
        // mkobj(2) next
        let second = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(second.meta().generation, Some(2));
        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }

    #[tokio::test]
    async fn predicate_filtering_should_handle_recreated_resources_with_same_generation() {
        use k8s_openapi::api::core::v1::Pod;

        let mkobj = |g: i32, uid: &str| {
            let p: Pod = serde_json::from_value(json!({
                "apiVersion": "v1",
                "kind": "Pod",
                "metadata": {
                    "name": "blog",
                    "namespace": "default",
                    "generation": Some(g),
                    "uid": uid,
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

        // Simulate: create (gen=1, uid=1) -> update (gen=1, uid=1) -> delete ->
        // create (gen=1, uid=2) -> delete -> create (gen=2, uid=3)
        let data = stream::iter([
            Ok(mkobj(1, "uid-1")),
            Ok(mkobj(1, "uid-1")),
            Ok(mkobj(1, "uid-2")),
            Ok(mkobj(2, "uid-3")),
        ]);
        let mut rx = pin!(PredicateFilter::new(
            data,
            predicates::generation,
            Config::default()
        ));

        // mkobj(1, uid-1) passed through
        let first = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(first.meta().generation, Some(1));
        assert_eq!(first.meta().uid.as_deref(), Some("uid-1"));

        // (no repeat mkobj(1, uid-1) - same UID and generation)
        // mkobj(1, uid-2) next - different UID detected
        let second = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(second.meta().generation, Some(1));
        assert_eq!(second.meta().uid.as_deref(), Some("uid-2"));

        // mkobj(2, uid-3) next
        let third = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(third.meta().generation, Some(2));
        assert_eq!(third.meta().uid.as_deref(), Some("uid-3"));

        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }

    #[tokio::test]
    async fn predicate_cache_ttl_evicts_expired_entries() {
        use k8s_openapi::api::core::v1::Pod;
        use std::time::Duration;

        let mkobj = |g: i32, uid: &str| {
            let p: Pod = serde_json::from_value(json!({
                "apiVersion": "v1",
                "kind": "Pod",
                "metadata": {
                    "name": "blog",
                    "namespace": "default",
                    "generation": Some(g),
                    "uid": uid,
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

        // Use a very short TTL for testing
        let config = Config {
            ttl: Duration::from_millis(50),
        };

        // Create a stream that we'll manually poll
        let data = stream::iter([
            Ok(mkobj(1, "uid-1")),
            Ok(mkobj(1, "uid-1")), // Same, should be filtered
            Ok(mkobj(1, "uid-1")), // After TTL, should pass through again
        ]);
        let mut rx = pin!(PredicateFilter::new(data, predicates::generation, config));

        // First object passes through
        let first = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(first.meta().generation, Some(1));
        assert_eq!(first.meta().uid.as_deref(), Some("uid-1"));

        // Second object is filtered (same gen, same uid, within TTL)
        // Third object should be filtered too if TTL hasn't expired

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Now the third object should pass through because cache entry expired
        let third = rx.next().now_or_never().unwrap().unwrap().unwrap();
        assert_eq!(third.meta().generation, Some(1));
        assert_eq!(third.meta().uid.as_deref(), Some("uid-1"));

        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }
}
