0.32.0 / 2020-04-XX
===================
  * Major `config` + `client` module refactor
  * `Config` is the new `Configuration` struct
  * `Client` is now just a configured `reqwest::Client` plus a `reqwest::Url`
  * implement `From<Config> for reqwest::ClientBuilder`
  * implement `TryFrom<Config> for Client`
  * `Client::try_default` or `Client::new` now recommended constructors
  * People parsing `~/.kube/config` must use the `KubeConfig` struct instead
  * `Reflector<K>` now only takes an `Api<K>` to construct (.params method)
  * `Informer<K>` now only takes an `Api<K>` to construct (.params method)
  * `Reflector` is now self-polls #151
  * `Reflector` now has basic signal handling #152
  * `Api::watch` no longer filters out error events (`next` -> `try_next`)
  * `Api::watch` returns `Result<WatchEvent>` rather than `WatchEvent`
  * `WatchEvent::Bookmark` added to enum
  * `ListParams::allow_bookmarks` added

0.31.0 / 2020-03-27
===================
  * Expose `config::Configuration` at root level
  * Add `Configuration::infer` as a recommended constructor
  * Rename `client::APIClient` to `client::Client`
  * Expose `client::Client` at root level
  * `Client` now implements `From<Configuration>`
  * Added comprehensive documentation on `Api`
  * Rename `config::KubeConfigLoader` -> `config::ConfigLoader`
  * removed `futures-timer` dependency for `tokio` (feature=timer)

0.30.0 / 2020-03-17
===================
  * Fix `#[kube(printcolumn)]` when `#[kube(apiextensions = "v1beta1")]`
  * Fix `#[kube(status)]` causing serializes of empty optional statuses

0.29.0 / 2020-03-12
===================
  * `Api::log` -> `Api::logs` (now matches `Resource::logs`)
  * `Object<FooSpec, FooStatus>` back for ad-hoc ser/de
  * kube-derive now derives `Debug` (requires `Debug` on spec struct)
  * kube-derive now allows multiple derives per file
  * `Api::create` now takes data `K` rather than bytes
  * `Api::replace` now takes data `K` rather than bytes
    - (note that `Resource::create` and `Resource::replace` still takes bytes)

0.28.1 / 2020-03-07
===================
  * `#[derive(CustomResource)]` now implements `::new` on the generated `Kind`
  * derived `Kind` now properly contains `TypeMeta` - #170

0.28.0 / 2020-03-05
===================
  * `RawApi` removed -> `Resource` added
  * `Resource` implements `k8s_openapi::Resource`
  * **ALL OBJECTS REMOVED** -> Depening on light version of `k8s-openapi` now
    - NB: should generally just mean a few import changes (+casings / unwraps)
  * `openapi` feature removed (light dependency mandatory now)
  * **LIBRARY WORKS WITH ALL `k8s_openapi` KUBERNETES OBJECTS**
  * `KubeObject` trait removed in favour of `Meta` trait
  * `Object<FooSpec, FooStatus>` removed -> types implementing `k8s_openapi::Resource` required instead
  * `kube-derive` crate added to derive this trait + other kubebuilder like codegen

0.27.0 / 2020-02-26
===================
 * `Reflector` + `Informer` moved from `kube::api` to `kube::runtime`
 * `Informer` now resets the version to 0 rather than dropping events - #134
   * Removed `Informer::init`, since it is now a no-op when building the `Informer`
 * Downgrade spurious log message when using service account auth

0.26.0 / 2020-02-25
===================
  * Fix a large percentage of EOFs from watches #146
  * => default timeout down to 290s from 300s
  * => `Reflector` now re-lists a lot less #146
  * Fix decoder panic with async-compression (probably) #144
  * `Informer::poll` can now be used with `TryStream`
  * Exposed `Config::read` and `Config::read_from` - #124
  * Fix typo on `Api::StatefulSet`
  * Fix typo on `Api::Endpoints`
  * Add `Api::v1CustomResourceDefinition` when on k8s >= 1.17
  * Renamed `Void` to `NotUsed`

0.25.0 / 2020-02-09
===================
  * initial rustls support #114 (some local kube config issues know #120)
  * crate does better version checking against openapi features - #106
  * initial `log_stream` support - #109

0.24.0 / 2020-01-26
===================
  * Add support for ServiceAccount, Role, ClusterRole, RoleBinding, Endpoint - #113 + #111
  * Upgrade k8s-openapi to 0.7 => breaking changes: https://github.com/Arnavion/k8s-openapi/blob/master/CHANGELOG.md#v070-2020-01-23

0.23.0 / 2019-12-31
===================
  * Bump tokio and reqwest to 0.2 and 0.10
  * Fix bug in `log` fetcher - #107
  * Temporarily allow invalid certs when testing on macosx - #105

0.22.2 / 2019-12-04
===================
  * Allow sharing Reflectors between threads - #97
  * Fix Reflector pararall lock issue (`poll` no longer blocks `state`)

0.22.1 / 2019-11-30
===================
  * Improve Reflector reset algorithm (clear history less)

0.22.0 / 2019-11-29
===================
  * Default watch timeouts changed to 300s everywhere
  * This increases efficiency of Informers and Reflectors by keeping the connection open longer.
  * However, if your Reflector relies on frequent polling you can set `timeout` or hide the `poll()` in a different context so it doesn't block your main work
  * Internal `RwLock` changed to a `futures::Mutex` for soundness / proper non-blocking - #94
  * blocking `Reflector::read()` renamed to `async Reflector::state()`
  * Expose `metadata.creation_timestamp` and `.deletion_timestamp` (behind openapi flag) - #93

0.21.0 / 2019-11-29
===================
  * All watch calls returns a stream of `WatchEvent` - #92
  * `Informer::poll` now returns a stream - #92

0.20.1 / 2019-11-21
===================
  * ObjectList now implements Iterator - #91
  * openapi feature no longer accidentally hardcoded to v1.15 feature - #90

0.19.0 / 2019-11-15
==================
  * kube::Error is now a proper error enum and not a Fail impl (thiserror)
  * soft-tokio dependency removed for futures-timer
  * gzip re-introduced

0.18.1 / 2019-11-11
==================
  * Fix unpinned gzip dependency breakage - #87

0.18.0 / 2019-11-07
==================
  * api converted to use async/await with 1.39.0 (primitively)
  * hyper upgraded to 0.10-alpha
  * synchronous sleep replaced with tokio timer
  * `Log` trait removed in favour of internal marker trait

0.17.0 / 2019-10-22
==================
  * Add support for oidc providerss with `auth-provider` w/o `access-token` - #70
  * Bump most dependencies to more recent versions
  * Expose custom client creation
  * Added support for `v1beta1Ingress`
  * Expose incluster_config::load_default_ns - #74

0.16.1 / 2019-08-09
==================
  * Add missing `uid` field on `ObjectMeta::ownerReferences`

0.16.0 / 2019-08-09
==================
  * Add `Reflector::get` and `Reflector::get_within` as cheaper getters
  * Add support for OpenShift kube configs with multiple CAs - via #64
  * Add missing `ObjectMeta::ownerReferences`
  * Reduced memory consumption during compile with `k8s-openapi@0.5.1` - #62

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
