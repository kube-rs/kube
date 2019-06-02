0.9.0 / 2019-06-02
==================
  * Status subresource api commands added to `Api`:
    - `patch_status`
    - `get_status`
    - `replace_status`
  ^ See `crd_openapi` or `crd_api` examples
  * Scale subresource commands added to `Api`:
    - `patch_scale`
    - `get_scale`
    - `replace_scale`
  ^ See `crd_openapi` example

0.8.0 / 2019-05-31
==================
  * Typed `Api` variant called `OpenApi` introduced (see crd_openapi example)
  * Revert `client.request` return type change (back to response only from pre-0.7.0 #28)
  * `delete` now returns `Either<Object<P, U>, ApiStatus> - for bug#32
  * `delete_collection` now returns `Either<ObjectList<Object<P, U>>, ApiStatus> - for bug#32
  * `Informer::new` renamed to `Informer::raw`
  * `Reflector::new` renamed to `Reflector::raw`
  * `Reflector::new` + `Informer::new` added for "openapi" compile time feature (does not require specifying the generic types)

0.7.0 / 2019-05-27
==================
  * Expose list/watch parameters #11
  * Many API struct renames:
    - `ResourceMap` -> `Cache`
    - `Resource` -> `Object`
    - `ResourceList` -> `ObjectList`
    - `ApiResource` -> `Api`
  * `ResourceType` has been removed in favour of `Api::v1Pod()` say
  * `Object::status` now wrapped in an `Option` (not present everywhere)
  * `ObjectList` exposed
  * Major API overhaul to support generic operations on `Object`
  * Api can be used to perform generic actions on resources:
    - `create`
    - `get`
    - `delete`
    - `watch`
    - `list`
    - `patch`
    - `replace`
    - `get_scale` (when scale subresource exists)
    - `patch_scale` (ditto)
    - `replace_scale` (ditto)
    - `get_status` (when status subresource exists)
    - `patch_status` (ditto)
    - `replace_status` (ditto)
  * crd_api example added to track the action api
  * Bunch of generic parameter structs exposed for common operations:
    - `ListParams` exposed
    - `DeleteParams` exposed
    - `PostParams` exposed
  * Errors from `Api` exposed in `kube::Error`:
    - `Error::api_error -> Option<ApiError>` exposed
    - Various other error types also in there (but awkward setup atm)
  * `client.request` now returns a tuple `(T, StatusCode)` (before only `T`)

0.6.0 / 2019-05-12
==================
  * Expose getter `Informer::version`
  * Exose ctor `Informer::from_version`
  * Expose more attributes in `Metadata`
  * `Informer::reset` convenience method added
  * `Informer::poll` no longer returns events straight
  * an `Informer` now caches `WatchEvent` elements into an internal queue
  * `Informer::pop` pops a single element from its internal queue
  * `Reflector::refresh` renamed to `Reflector::reset` (matches `Informer`)
  * `Void` type added so we can use `Reflector<ActualSpec, Void>`
    - removes need for Spec/Status structs:
    - `ReflectorSpec`, `ReflectorStatus` removed
    - `InformerSpec`, `InformerStatus` removed
    - `ResourceSpecMap`, `ResourceStatusMap` removed
  * `WatchEvents` removed
  * `WatchEvent` exposed, and now wraps `Resource<T, U>``

0.5.0 / 2019-05-09
==================
  * added `Informer` struct dedicated to handling events
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
