0.5.0 / 2019-05-09
==================
  * `Informer` struct to handle event
  * Reflectors no longer cache `events` - see #6

0.4.0 / 2019-05-09
==================
  * ResourceMap now contains the full Resource<T,U> struct rather than a tuple as the value. => `value.metadata` is available in the cache.
  * Reflectors now also cache `events` to allow apps to handle them

0.3.0 / 2019-05-09
==================
  * `Named` trait removed (inferring from metadata.name now)
  * Reflectors now take two type parameters (unless you use `ReflectorSpec` or `ReflectorStatus`) - see examples for usage
  * Native kube types supported via `ApiResource`
  * Some native kube resources have easy converters to `ApiResource`
