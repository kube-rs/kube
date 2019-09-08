0.16.0 / 2019-08-XX
==================
  * Add `Reflector::get` and `Reflector::get_within` as cheaper getters
  * Add support for OpenShift kube configs with multiple CAs - via #64
  * Add missing `ObjectMeta::ownerReferences`
  * TODO: will be published when k8s-openapi@0.5.1 is available

0.15.1 / 2019-08-18
==================
  * Fix compile issue on `1.37.0` with `Utc` serialization
  * Fix `Void` not having `Serialize` derive

0.15.0 / 2019-08-11
==================
  * Added support for `v1Job` resources - via #58
  * Added support for `v1Namespace`, `v1DaemonSet`, `v1ReplicaSet`, `v1PersistentVolumeClaim`, `v1PersistentVolume`, `v1ResourceQuota`, `v1HorizontalPodAutoscaler` - via #59
  * Added support for `v1beta1CronJob`, `v1ReplicationController`, `v1VolumeAttachment`, `v1NetworkPolicy` - via #60
  * `k8s-openapi` optional dependency bumped to `0.5.0` (for kube 1.14 structs)

0.14.0 / 2019-08-03
==================
  * `Reflector::read` now returns a `Vec<K>`` rather than a `Vec<(name, K)>`:
    This fixes an unsoundness bug internally - #56 via @gnieto

0.13.0 / 2019-07-22
==================
  * Experimental oauth2 support for some providers - via #44 :
    - a big cherry-pick from various prs upstream originally for GCP
    - EKS works with setup in https://github.com/clux/kube-rs/pull/20#issuecomment-511767551

0.12.0 / 2019-07-18
==================
  * Added support for `Log` subresource - via #50
  * Added support for `v1ConfigMap` with example - via #49
  * Demoted some spammy info messages from Reflector

0.11.0 / 2019-07-10
==================
  * Added `PatchParams` with `PatchStrategy` to allow arbitrary patch types - #24 via @ragne
  * `Event` renamed to `v1Event` to match non-slowflake type names
  * `v1Service` support added
  * Added `v1Secret` snowflake type and a `secret_reflector` example

0.10.0 / 2019-06-03
==================
  * `Api<P, U>` is now `Api<K>` for some `KubeObject` K:
    - Big change to allow snowflake objects (#35) - but also slightly nicer
    - You want aliases `type Pod = Object<PodSpec, PodStatus>`
    - This gives you the required `KubeObject` trait impl for free
  * Added `Event` native type to prove snowflakes can be handled - #35

  * `ApiStatus` renamed to `Status` to match kube api conventions #36
  * Rename `Metadata` to `ObjectMeta` #36
  * Added `ListMeta` for `ObjectList` and `Status` #36
  * Added `TypeMeta` object which is flattened onto `Object`, so:
    - `o.types.kind` rather than `o.kind`
    - `o.types.version` rather than `o.version`

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
