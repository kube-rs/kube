0.4.0 / UNRELEASED
==================
  * ResourceMap now contains the full Resource<T,U> struct rather than a tuple as the value. => `value.metadata` is available in the cache.

0.3.0 / 2019-05-09
==================
  * `Named` trait removed (inferring from metadata.name now)
  * Reflectors now take two type parameters (unless you use `ReflectorSpec` or `ReflectorStatus`) - see examples for usage
  * Native kube types supported via `ApiResource`
  * Some native kube resources have easy converters to `ApiResource`
