<!-- next-header -->
UNRELEASED
===================
 * see https://github.com/kube-rs/kube-rs/compare/0.58.1...master
 * `kube` client connects to kubernetes via cluster dns when using `rustls` - #587 via #597
 * `kube` client now works with `rustls` feature in cluster - #153 via #597
 * `kube-core` added `CrdExtensions::crd_name` method (implemented by `kube-derive`) - #583

0.58.1 / 2021-07-06
===================
 * `kube-runtime`: fix non-unix builds - [#582](https://github.com/kube-rs/kube-rs/issues/582)

0.58.0 / 2021-07-05
===================
 * `kube`: `BREAKING`: subresource marker traits renamed conjugation: `Log`, `Execute`, `Attach`, `Evict` (previously `Logging`, `Executable`, `Attachable`, `Evictable`) - [#536](https://github.com/kube-rs/kube-rs/issues/536) via [#560](https://github.com/kube-rs/kube-rs/issues/560)
 * `kube-derive` added `#[kube(category)]` attr to set [CRD categories](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#categories) - [#559](https://github.com/kube-rs/kube-rs/issues/559)
 * `kube-runtime` added `finalizer` helper [#291](https://github.com/kube-rs/kube-rs/issues/291) via [#475](https://github.com/kube-rs/kube-rs/issues/475)
 * `kube-runtime` added tracing for why reconciliations happened [#457](https://github.com/kube-rs/kube-rs/issues/457) via [#571](https://github.com/kube-rs/kube-rs/issues/571)
 * `kube-runtime` added `Controller::reconcile_all_on` to allow scheduling all objects for reconciliation [#551](https://github.com/kube-rs/kube-rs/issues/551) via [#555](https://github.com/kube-rs/kube-rs/issues/555)
 * `kube-runtime` added `Controller::graceful_shutdown_on` for shutting down the `Controller` while waiting for running reconciliations to finish - [#552](https://github.com/kube-rs/kube-rs/issues/552) via [#573](https://github.com/kube-rs/kube-rs/issues/573)
  - BREAKING: `controller::applier` now starts a graceful shutdown when the `queue` terminates
  - BREAKING: `scheduler` now shuts down immediately when `requests` terminates, rather than waiting for the pending reconciliations to drain
 * `kube-runtime` added tracking for reconciliation reason
  - Added: `Controller::owns_with` and `Controller::watches_with` to pass a `dyntype` argument for dynamic `Api`s - [#575](https://github.com/kube-rs/kube-rs/issues/575)
  - BREAKING: `Controller::owns` signature changed to not allow `DynamicType`s
  - BREAKING: `controller::trigger_*` now returns a `ReconcileRequest` rather than `ObjectRef`. The `ObjectRef` can be accessed via the `obj_ref` field

### Known Issues
- Api::replace can fail to unset list values with k8s-openapi 0.12 [#581](https://github.com/kube-rs/kube-rs/issues/581)

0.57.0 / 2021-06-16
===================
 * `kube`: custom clients now respect default namespaces - fixes [#534](https://github.com/kube-rs/kube-rs/issues/534) via [#544](https://github.com/kube-rs/kube-rs/issues/544)
  - BREAKING: custom clients via `Client::new` must pass `config.default_namespace` as 2nd arg
 * `kube`: Added `CustomResourceExt` trait for `kube-derive` - [#497](https://github.com/kube-rs/kube-rs/issues/497) via [#545](https://github.com/kube-rs/kube-rs/issues/545)
  - BREAKING: `kube-derive` users must import `kube::CustomResourceExt` (or `kube::core::crd::v1beta1::CustomResourceExt` if using legacy `#[kube(apiextensions = "v1beta1")]`) to use generated methods `Foo::crd` or `Foo::api_resource`
  - BREAKING: `k8s_openapi` bumped to [0.12.0](https://github.com/Arnavion/k8s-openapi/releases/tag/v0.12.0) - [#531](https://github.com/kube-rs/kube-rs/pull/531)
    * Generated structs simplified + `Resource` trait expanded
    * Adds support for kubernetes `v1_21`
    * Contains bugfix for [kubernetes#102159](https://github.com/kubernetes/kubernetes/pull/102159)
 * `kube` resource plurals is no longer inferred from `k8s-openapi` structs - [#284](https://github.com/kube-rs/kube-rs/issues/284) via [#556](https://github.com/kube-rs/kube-rs/issues/556)
  - BREAKING: `kube::Resource` trait now requires a `plural` implementation

### Known Issues
- Api::replace can fail to unset list values with k8s-openapi 0.12 [#581](https://github.com/kube-rs/kube-rs/issues/581)

0.56.0 / 2021-06-05
===================
 * `kube`: added `Api::default_namespaced` - [#209](https://github.com/kube-rs/kube-rs/issues/209) via [#534](https://github.com/kube-rs/kube-rs/issues/534)
 * `kube`: added `config` feature - [#533](https://github.com/kube-rs/kube-rs/issues/533) via [#535](https://github.com/kube-rs/kube-rs/issues/535)
 * `kube`: BREAKING: moved `client::discovery` module to `kube::discovery` and rewritten module [#538](https://github.com/kube-rs/kube-rs/issues/538)
  - `discovery`: added `oneshot` helpers for quick selection of recommended resources / kinds [#538](https://github.com/kube-rs/kube-rs/issues/538)
  - `discovery`: moved `ApiResource` and `ApiCapabilities` (result of discovery) to `kube_core::discovery`
  - BREAKING: removed internal `ApiResource::from_apiresource`

 * `kube::Client` is now configurable with layers using `tower-http` [#539](https://github.com/kube-rs/kube-rs/issues/539) via [#540](https://github.com/kube-rs/kube-rs/issues/540)
  - three new examples added: [`custom_client`](./examples/custom_client.rs), [`custom_client_tls`](./examples/custom_client_tls.rs) and [`custom_client_trace`](./examples/custom_client_trace.rs)
  - Big feature streamlining, big service and layer restructuring, dependency restructurings
  - Changes can hit advanced users, but unlikely to hit base use cases with `Api` and `Client`.
  - In depth changes broken down below:

### TLS Enhancements

- Add `kube::client::ConfigExt` extending `Config` for custom `Client`. This includes methods to configure TLS connection when building a custom client [#539](https://github.com/kube-rs/kube-rs/issues/539)
  - `native-tls`: `Config::native_tls_https_connector` and `Config::native_tls_connector`
  - `rustls-tls`: `Config::rustls_https_connector` and `Config::rustls_client_config`
- Remove the requirement of having `native-tls` or `rustls-tls` enabled when `client` is enabled. Allow one, both or none.
  - When both, the default Service will use `native-tls` because of [#153](https://github.com/kube-rs/kube-rs/issues/153). `rustls` can be still used with a custom client. Users will have an option to configure TLS at runtime.
  - When none, HTTP connector is used.
- Remove TLS features from `kube-runtime`
  - **BREAKING**: Features must be removed if specified
- Remove `client` feature from `native-tls` and `rust-tls` features
  - `config` + `native-tls`/`rustls-tls` can be used independently, e.g., to create a simple HTTP client
  - **BREAKING**: `client` feature must be added if `default-features = false`

### Layers
- `ConfigExt::base_uri_layer` (`BaseUriLayer`) to set cluster URL (#539)
- `ConfigExt::auth_layer` that returns optional layer to manage `Authorization` header (#539)
- `gzip`: Replaced custom decompression module with [`DecompressionLayer`](https://docs.rs/tower-http/0.1.0/tower_http/decompression/index.html) from `tower-http` (#539)
- Replaced custom `LogRequest` with [`TraceLayer`](https://docs.rs/tower-http/0.1.0/tower_http/trace/index.html) from `tower-http` (#539)
  - Request body is no longer shown
- Basic and Bearer authentication using `AddAuthorizationLayer` (borrowing from https://github.com/tower-rs/tower-http/pull/95 until released)
- **BREAKING**: Remove `headers` from `Config`. Injecting arbitrary headers is now done with a layer on a custom client.

### Dependency Changes

- Remove `static_assertions` since it's no longer used
- Replace `tokio_rustls` with `rustls` and `webpki` since we're not using `tokio_rustls` directly
- Replace uses of `rustls::internal::pemfile` with `rustls-pemfile`
- Remove `url` and always use `http::Uri`
  - **BREAKING**: `Config::cluster_url` is now `http::Uri`
  - **BREAKING**: `Error::InternalUrlError(url::ParseError)` and `Error::MalformedUrl(url::ParseError)` replaced by `Error::InvalidUri(http::uri::InvalidUri)`


0.55.0 / 2021-05-21
===================
 * `kube`: `client` feature added (default-enabled) - [#528](https://github.com/kube-rs/kube-rs/issues/528)
 * `kube`: `PatchParams` force now only works with `Patch::Apply` [#528](https://github.com/kube-rs/kube-rs/issues/528)
 * `kube`: `api` `discovery` module now uses a new `ApiResource` struct [#495](https://github.com/kube-rs/kube-rs/issues/495) + [#482](https://github.com/kube-rs/kube-rs/issues/482)
 * `kube`: `api` BREAKING: `DynamicObject` + `Object` now takes an `ApiResource` rather than a `GroupVersionKind`
 * `kube`: `api` BREAKING: `discovery` module's `Group` renamed to `ApiGroup`
 * `kube`: `client` BREAKING: `kube::client::Status` moved to `kube::core::Status` (accidental, re-adding in 0.56)
 * `kube-core` crate factored out of `kube` to reduce dependencies - [#516](https://github.com/kube-rs/kube-rs/issues/516) via [#517](https://github.com/kube-rs/kube-rs/issues/517) + [#519](https://github.com/kube-rs/kube-rs/issues/519) + [#522](https://github.com/kube-rs/kube-rs/issues/522) + [#528](https://github.com/kube-rs/kube-rs/issues/528) + [#530](https://github.com/kube-rs/kube-rs/issues/530)
 * `kube`: `kube::Service` removed to allow `kube::Client` to take an abritrary `Service<http::Request<hyper::Body>>` - [#532](https://github.com/kube-rs/kube-rs/issues/532)

0.54.0 / 2021-05-19
===================
 * yanked 30 minutes after release due to [#525](https://github.com/kube-rs/kube-rs/issues/525)
 * changes lifted to 0.55.0

0.53.0 / 2021-05-15
===================
 * `kube`: `admission` controller module added under feature - [#477](https://github.com/kube-rs/kube-rs/issues/477) via [#484](https://github.com/kube-rs/kube-rs/issues/484) + fixes in [#488](https://github.com/kube-rs/kube-rs/issues/488) [#498](https://github.com/kube-rs/kube-rs/issues/498) [#499](https://github.com/kube-rs/kube-rs/issues/499) + [#507](https://github.com/kube-rs/kube-rs/issues/507) + [#509](https://github.com/kube-rs/kube-rs/issues/509)
 * `kube`: `config` parsing of pem blobs now resilient against missing newlines - [#504](https://github.com/kube-rs/kube-rs/issues/504) via [#505](https://github.com/kube-rs/kube-rs/issues/505)
 * `kube`: `discovery` module added to simplify dynamic api usage - [#491](https://github.com/kube-rs/kube-rs/issues/491)
 * `kube`: `api` BREAKING: `DynamicObject::namespace` renamed to `::within` - [#502](https://github.com/kube-rs/kube-rs/issues/502)
 * `kube`: `api` BREAKING: added `ResourceExt` trait moving the getters from `Resource` trait - [#486](https://github.com/kube-rs/kube-rs/issues/486)
 * `kube`: `api` added a generic interface for subresources via `Request` - [#487](https://github.com/kube-rs/kube-rs/issues/487)
 * `kube`: `api` fix bug in `PatchParams::dry_run` not being serialized correctly - [#511](https://github.com/kube-rs/kube-rs/issues/511)

### 0.53.0 Migration Guide
The most likely issue you'll run into is from `kube` when using `Resource` trait which has been split:

```diff
+use kube::api::ResouceExt;
-    let name = Resource::name(&foo);
-    let ns = Resource::namespace(&foo).expect("foo is namespaced");
+    let name = ResourceExt::name(&foo);
+    let ns = ResourceExt::namespace(&foo).expect("foo is namespaced");
```

0.52.0 / 2021-03-31
===================
 * `kube-derive`: allow overriding `#[kube(plural)]` and `#[kube(singular)]` - [#458](https://github.com/kube-rs/kube-rs/issues/458) via [#463](https://github.com/kube-rs/kube-rs/issues/463)
 * `kube`: added tracing instrumentation for io operations in `kube::Api` - [#455](https://github.com/kube-rs/kube-rs/issues/455)
 * `kube`: `DeleteParams`'s `Preconditions` is now public - [#459](https://github.com/kube-rs/kube-rs/issues/459) via [#460](https://github.com/kube-rs/kube-rs/issues/460)
 * `kube`: remove dependency on duplicate `derive_accept_key` for `ws` - [#452](https://github.com/kube-rs/kube-rs/issues/452)
 * `kube`: Properly verify websocket keys in `ws` handshake - [#447](https://github.com/kube-rs/kube-rs/issues/447)
 * `kube`: BREAKING: removed optional, and deprecated `runtime` module - [#454](https://github.com/kube-rs/kube-rs/issues/454)
 * `kube`: BREAKING: `ListParams` bookmarks default enabled - [#226](https://github.com/kube-rs/kube-rs/issues/226) via [#445](https://github.com/kube-rs/kube-rs/issues/445)
   - renames member `::allow_bookmarks` to `::bookmarks`
   - `::default()` sets `bookmark` to `true` to avoid bad bad defaults [#219](https://github.com/kube-rs/kube-rs/issues/219)
   - method `::allow_bookmarks()` replaced by `::disable_bookmarks()`
 * `kube`: `DynamicObject` and `GroupVersionKind` introduced for full dynamic object support
 * `kube-runtime`: watchers/reflectors/controllers can be used with dynamic objects from api discovery
 * `kube`: Pluralisation now only happens for `k8s_openapi` objects by default [#481](https://github.com/kube-rs/kube-rs/issues/481)
   - inflector dependency removed [#471](https://github.com/kube-rs/kube-rs/issues/471)
   - added internal pluralisation helper for `k8s_openapi` objects
 * `kube`: BREAKING: Restructuring of low level `Resource` request builder [#474](https://github.com/kube-rs/kube-rs/issues/474)
   - `Resource` renamed to `Request` and requires only a `path_url` to construct
 * `kube`: BREAKING: Mostly internal `Meta` trait revamped to support dynamic types
   - `Meta` renamed to `kube::Resource` to mimic `k8s_openapi::Resource` [#478](https://github.com/kube-rs/kube-rs/issues/478)
   - The trait now takes an optional associated type for runtime type info: `DynamicType` [#385](https://github.com/kube-rs/kube-rs/issues/385)
   - `Api::all_with` + `Api::namespaced_with` added for querying with dynamic families
   - see `dynamic_watcher` + `dynamic_api` for example usage
 * `kube-runtime`: BREAKING: lower level interface changes as a result of `kube::api::Meta` trait:
  - THESE SHOULD NOT AFFECT YOU UNLESS YOU ARE IMPLEMENTING / CUSTOMISING LOW LEVEL TYPES DIRECTLY
  - `ObjectRef` now generic over `kube::Resource` rather than `RuntimeResource`
  - `reflector::{Writer, Store}` takes a `kube::Resource` rather than a `k8s_openapi::Resource`
 * `kube-derive`: BREAKING: Generated type no longer generates `k8s-openapi` traits
  - This allows correct pluralisation via `#[kube(plural = "mycustomplurals")]` [#467](https://github.com/kube-rs/kube-rs/issues/467) via [#481](https://github.com/kube-rs/kube-rs/issues/481)

### 0.52.0 Migration Guide
While we had a few breaking changes. Most are to low level internal interfaces and should not change much, but some changes you might need to make:

#### kube
- if using the old, low-level `kube::api::Resource`, please consider the easier `kube::Api`, or look at tests in `request.rs` or `typed.rs` if you need the low level interface
- search replace `kube::api::Meta` with `kube::Resource` if used - trait was renamed
- if implementing the trait, add `type DynamicType = ();` to the impl
- remove calls to `ListParams::allow_bookmarks` (allow default)
- handle `WatchEvent::Bookmark` or set `ListParams::disable_bookmarks()`
- look at examples if replacing the long deprecated legacy runtime

#### kube-derive
The following constants from `k8s_openapi::Resource` no longer exist. Please `use kube::Resource` and:
- replace `Foo::KIND` with `Foo::kind(&())`
- replace `Foo::GROUP` with `Foo::group(&())`
- replace `Foo::VERSION` with `Foo::version(&())`
- replace `Foo::API_VERSION` with `Foo::api_version(&())`

0.51.0 / 2021-02-28
===================
 * `kube` `Config` now allows arbirary extension objects - [#425](https://github.com/kube-rs/kube-rs/issues/425)
 * `kube` `Config` now allows multiple yaml documents per kubeconfig - [#440](https://github.com/kube-rs/kube-rs/issues/440) via [#441](https://github.com/kube-rs/kube-rs/issues/441)
 * `kube-derive` now more robust and is using `darling` - [#435](https://github.com/kube-rs/kube-rs/issues/435)
 * docs improvements to patch + runtime

0.50.1 / 2021-02-17
===================
 * bug: fix oidc auth provider - [#424](https://github.com/kube-rs/kube-rs/issues/424) via [#419](https://github.com/kube-rs/kube-rs/issues/419)

0.50.0 / 2021-02-10
===================
 * feat: added support for stacked kubeconfigs - [#132](https://github.com/kube-rs/kube-rs/issues/132) via [#411](https://github.com/kube-rs/kube-rs/issues/411)
 * refactor: authentication logic moved out of `kube::config` and into into `kube::service` - [#409](https://github.com/kube-rs/kube-rs/issues/409)
  - BREAKING: `Config::get_auth_header` removed
 * refactor: remove `hyper` dependency from `kube::api` - [#410](https://github.com/kube-rs/kube-rs/issues/410)
 * refactor: `kube::Service` simpler auth and gzip handling - [#405](https://github.com/kube-rs/kube-rs/issues/405) + [#408](https://github.com/kube-rs/kube-rs/issues/408)

0.49.0 / 2021-02-08
===================
 * dependency on `reqwest` + removed in favour of `hyper` + `tower` [#394](https://github.com/kube-rs/kube-rs/pull/394)
   - refactor: `kube::Client` now uses `kube::Service` (a `tower::Service<http::Request<hyper::Body>>`) instead of `reqwest::Client` to handle all requests
   - refactor: `kube::Client` now uses a `tokio_util::codec` for internal buffering
   - refactor: `async-tungstenite` ws feature dependency replaced with `tokio-tungstenite`. `WebSocketStream` is now created from a connection upgraded with `hyper`
   - refactor: `oauth2` module for GCP OAuth replaced with optional `tame-oauth` dependency
   - BREAKING: GCP OAuth is now opt-in (`oauth` feature). Note that GCP provider with command based token source is supported by default.
   - BREAKING: Gzip decompression is now opt-in (`gzip` feature) because Kubernetes does not have compression enabled by default yet and this feature requires extra dependencies. [#399](https://github.com/kube-rs/kube-rs/pull/399)
   - BREAKING: `Client::new` now takes a `Service` instead of `Config` [#400](https://github.com/kube-rs/kube-rs/pull/400). Allows custom service for features not supported out of the box and testing. To create a `Client` from `Config`, use `Client::try_from` instead.
   - BREAKING: Removed `Config::proxy`. Proxy is no longer supported out of the box, but it should be possible by using a custom Service.
   - fix: Refreshable token from auth provider not refreshing
   - fix: Panic when loading config with non-GCP provider [#238](https://github.com/kube-rs/kube-rs/issues/238)
 * feat: subresource support added for `Evictable` types (marked for `Pod`) - [#393](https://github.com/kube-rs/kube-rs/pull/393)
 * `kube`: subresource marker traits renamed to `Loggable`, `Executable`, `Attachable` (previously `LoggingObject`, `ExecutingObject`, `AttachableObject`) - [#395](https://github.com/kube-rs/kube-rs/pull/395)
 * `examples` showcasing `kubectl cp` like behaviour [#381](https://github.com/kube-rs/kube-rs/issues/381) via [#392](https://github.com/kube-rs/kube-rs/pull/392)

0.48.0 / 2021-01-23
===================
  * bump `k8s-openapi` to `0.11.0` - [#388](https://github.com/kube-rs/kube-rs/pull/388)
  * breaking: `kube`: no longer necessary to serialize patches yourself - [#386](https://github.com/kube-rs/kube-rs/pull/386)
    - `PatchParams` removes `PatchStrategy`
    - `Api::patch*` methods now take an enum `Patch` type
    - optional `jsonpatch` feature added for `Patch::Json`

0.47.0 / 2021-01-06
===================
  * chore: upgrade `tokio` to `1.0` - [#363](https://github.com/kube-rs/kube-rs/pull/363)
    * BREAKING: This requires the whole application to upgrade to `tokio` 1.0 and `reqwest` to 0.11.0
  * docs: fix broken documentation in `kube` 0.46.0 [#367](https://github.com/kube-rs/kube-rs/pull/367)
  * bug: `kube`: removed panics from `ws` features, fix `rustls` support + improve docs [#369](https://github.com/kube-rs/kube-rs/issues/369) via [#370](https://github.com/kube-rs/kube-rs/pull/370) + [#373](https://github.com/kube-rs/kube-rs/pull/373)
  * bug: `AttachParams` now fixes owned method chaining (slightly breaks from 0.46 if using &mut ref before) - [#364](https://github.com/kube-rs/kube-rs/pull/364)
  * feat: `AttachParams::interactive_tty` convenience method added - [#364](https://github.com/kube-rs/kube-rs/pull/364)
  * bug: fix `Runner` (and thus `Controller` and `applier`) not waking correctly when starting new tasks - [#375](https://github.com/kube-rs/kube-rs/pull/375)

0.46.1 / 2021-01-06
===================
  * maintenance release for 0.46 (last supported tokio 0.2 release) from `tokio02` branch
  * bug backport: fix `Runner` (and thus `Controller` and `applier`) not waking correctly when starting new tasks - [#375](https://github.com/kube-rs/kube-rs/pull/375)

0.46.0 / 2021-01-02
===================
  * feat: `kube` now has __optional__ websocket support with `async_tungstenite` under `ws` and `ws-*-tls` features [#360](https://github.com/kube-rs/kube-rs/pull/360)
  * feat: `AttachableObject` marker trait added and implemented for `k8s_openapi::api::core::v1::Pod` [#360](https://github.com/kube-rs/kube-rs/pull/360)
  * feat: `AttachParams` added for `Api::exec` and `Api::attach` for `AttachableObject`s [#360](https://github.com/kube-rs/kube-rs/pull/360)
  * examples: `pod_shell`, `pod_attach`, `pod_exec` demonstrating the new features [#360](https://github.com/kube-rs/kube-rs/pull/360)

0.45.0 / 2020-12-26
===================
  * feat: `kube-derive` now has a default enabled `schema` feature
    * allows opting out of `schemars` dependency for handwriting crds - [#355](https://github.com/kube-rs/kube-rs/issues/355)
  * breaking: `kube-derive` attr `struct_name` renamed to `struct` - [#359](https://github.com/kube-rs/kube-rs/pull/359)
  * docs: improvements on `kube`, `kube-runtime`, `kube-derive`

0.44.0 / 2020-12-23
===================
  * feat: `kube-derive` now generates openapi v3 schemas and is thus usable with v1 `CustomResourceDefinition` - [#129](https://github.com/kube-rs/kube-rs/issues/129) and [#264](https://github.com/kube-rs/kube-rs/issues/264) via [#348](https://github.com/kube-rs/kube-rs/pull/348)
    * BREAKING: `kube-derive` types now require `JsonSchema` derived via `schemars` libray (not breaking if going to 0.45.0)
  * feat: `kube_runtime::controller`: now reconciles objects in parallel - [#346](https://github.com/kube-rs/kube-rs/issues/346)
    * BREAKING: `kube_runtime::controller::applier` now requires that the `reconciler`'s `Future` is `Unpin`,
                `Box::pin` it or submit it to a runtime if this is not acceptable
    * BREAKING: `kube_runtime::controller::Controller` now requires that the `reconciler`'s `Future` is `Send + 'static`,
                use the low-level `applier` interface instead if this is not acceptable
  * bug: `kube-runtime`: removed accidentally included `k8s-openapi` default features (you have to opt in to them yourself)
  * feat: `kube`: `TypeMeta` now derives additionally `Debug, Eq, PartialEq, Hash`
  * bump: `k8s-openapi` to `0.10.0` - [#330](https://github.com/kube-rs/kube-rs/pull/330)
  * bump: `serde_yaml` - [#349](https://github.com/kube-rs/kube-rs/issues/349)
  * bump: `dirs` to `dirs-next` - [#340](https://github.com/kube-rs/kube-rs/pull/340)

0.43.0 / 2020-10-08
===================
  * bug: `kube-derive` attr `#[kube(shortname)]` now working correctly
  * bug: `kube-derive` now working with badly cased existing types - [#313](https://github.com/kube-rs/kube-rs/issues/313)
  * missing: `kube` now correctly exports `config::NamedAuthInfo` - [#323](https://github.com/kube-rs/kube-rs/pull/323)
  * feat: `kube`: expose `Config::get_auth_header` for istio use cases - [#322](https://github.com/kube-rs/kube-rs/issues/322)
  * feat: `kube`: local config now tackles gcloud auth exec params - [#328](https://github.com/kube-rs/kube-rs/pull/328) and [#84](https://github.com/kube-rs/kube-rs/issues/84)
  * `kube-derive` now actually requires GVK (in particular `#[kube(kind = "Foo")]` which we sometimes inferred earlier, despite documenting the contrary)

0.42.0 / 2020-09-10
===================
  * bug: `kube-derive`'s `Default` derive now sets typemeta correctly - [#315](https://github.com/kube-rs/kube-rs/issues/315)
  * feat: `ListParams` now supports `continue_token` and `limit` - [#320](https://github.com/kube-rs/kube-rs/pull/320)

0.41.0 / 2020-09-10
===================
  * yanked release. failed publish.

0.40.0 / 2020-08-17
===================
  * `DynamicResource::from_api_resource` added to allow apiserver returned resources - [#305](https://github.com/kube-rs/kube-rs/pull/305) via [#301](https://github.com/kube-rs/kube-rs/pull/301)
  * `Client::list_api_groups` added
  * `Client::list_ap_group_resources` added
  * `Client::list_core_api_versions` added
  * `Client::list_core_api_resources` added
  * `kube::DynamicResource` exposed at top level
  * Bug: `PatchParams::default_apply()` now requires a manager and renamed to `PatchParams::apply(manager: &str)` for [#300](https://github.com/kube-rs/kube-rs/issues/300)
  * Bug: `DeleteParams` no longer missing for `Api::delete_collection` - [#53](https://github.com/kube-rs/kube-rs/issues/53)
  * Removed paramter `ListParams::include_uninitialized` deprecated since 1.14
  * Added optional `PostParams::field_manager` was missing for `Api::create` case

0.39.0 / 2020-08-05
===================
  * Bug: `ObjectRef` tweak in `kube-runtime` to allow controllers triggering across cluster and namespace scopes - [#293](https://github.com/kube-rs/kube-rs/issues/293) via [#294](https://github.com/kube-rs/kube-rs/pull/294)
  * Feature: `kube` now has a `derive` feature which will re-export `kube::CustomResource` from `kube-derive::CustomResource`.
  * Examples: revamp examples for `kube-runtime` - [#201](https://github.com/kube-rs/kube-rs/issues/201)

0.38.0 / 2020-07-23
===================
  * Marked `kube::runtime` module as deprecated - [#281](https://github.com/kube-rs/kube-rs/issues/281)
  * `Config::timeout` can now be overridden to `None` (with caveats) [#280](https://github.com/kube-rs/kube-rs/pull/280)
  * Bug: reflector stores could have multiple copies inside datastore - [#286](https://github.com/kube-rs/kube-rs/issues/286)
     - `dashmap` backend Store driver downgraded - [#286](https://github.com/kube-rs/kube-rs/issues/286)
     - `Store::iter` temporarily removed
  * Bug: Specialize WatchEvent::Bookmark so they can be deserialized - [#285](https://github.com/kube-rs/kube-rs/issues/285)
  * Docs: Tons of docs for kube-runtime

0.37.0 / 2020-07-20
===================
  * Bump `k8s-openapi` to `0.9.0`
  * All runtime components now require `Sync` objects
  * reflector/watcher/Controller streams can be shared in threaded environments

0.36.0 / 2020-07-19
===================
  * https://gitlab.com/teozkr/kube-rt/ merged in for a new `kube-runtime` crate [#258](https://github.com/kube-rs/kube-rs/pull/258)
  * `Controller<K>` added ([#148](https://github.com/kube-rs/kube-rs/issues/148) via [#258](https://github.com/kube-rs/kube-rs/pull/258))
  * `Reflector` api redesigned ([#102](https://github.com/kube-rs/kube-rs/issues/102) via [#258](https://github.com/kube-rs/kube-rs/pull/258))
  * Migration release for `Informer` -> `watcher` + `Reflector` -> `reflector`
  * `kube::api::CustomResource` removed in favour of `kube::api::Resource::dynamic`
  * `CrBuilder` removed in favour of `DynamicResource` (with new error handling)
  * support level bumped to beta

0.35.1 / 2020-06-18
===================
  * Fix in-cluster Client when using having multiple certs in the chain - [#251](https://github.com/kube-rs/kube-rs/issues/251)

0.35.0 / 2020-06-15
===================
  * `Config::proxy` support added - [#246](https://github.com/kube-rs/kube-rs/pull/246)
  * `PartialEq` can be derived with `kube-derive` - [#242](https://github.com/kube-rs/kube-rs/pull/242)
  * Windows builds no longer clashes with runtime - [#240](https://github.com/kube-rs/kube-rs/pull/240)
  * Rancher hosts (with path specifiers) now works - [#244](https://github.com/kube-rs/kube-rs/issues/244)

0.34.0 / 2020-05-08
===================
  * Bump `k8s-openapi` to `0.8.0`
  * `Config::from_cluster_env` <- renamed from `Config::new_from_cluster_env`
  * `Config::from_kubeconfig` <- renamed from `Config::new_from_kubeconfig`
  * `Config::from_custom_kubeconfig` added - [#236](https://github.com/kube-rs/kube-rs/pull/236)
  * Majorly overhauled error handlind in config module - [#237](https://github.com/kube-rs/kube-rs/pull/237)

0.33.0 / 2020-04-27
===================
  * documentation fixes for `Api::patch`
  * Config: add automatic token refresh - [#72](https://github.com/kube-rs/kube-rs/issues/72) / [#224](https://github.com/kube-rs/kube-rs/issues/224) / [#234](https://github.com/kube-rs/kube-rs/pull/234)

0.32.1 / 2020-04-15
===================
  * add missing tokio `signal` feature as a dependency
  * upgrade all dependencies, including minor bumps to rustls and base64

0.32.0 / 2020-04-10
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
  * `Informer::init_from` -> `Informer::set_version`
  * `Reflector` now self-polls [#151](https://github.com/kube-rs/kube-rs/issues/151) + handles signals [#152](https://github.com/kube-rs/kube-rs/issues/152)
  * `Reflector::poll` made private in favour of `Reflector::run`
  * `Api::watch` no longer filters out error events (`next` -> `try_next`)
  * `Api::watch` returns `Result<WatchEvent>` rather than `WatchEvent`
  * `WatchEvent::Bookmark` added to enum
  * `ListParams::allow_bookmarks` added
  * `PatchParams::default_apply` ctor added
  * `PatchParams` builder mutators: `::force` and `::dry_run` added

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
  * derived `Kind` now properly contains `TypeMeta` - [#170](https://github.com/kube-rs/kube-rs/issues/170)

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
 * `Informer` now resets the version to 0 rather than dropping events - [#134](https://github.com/kube-rs/kube-rs/issues/134)
   * Removed `Informer::init`, since it is now a no-op when building the `Informer`
 * Downgrade spurious log message when using service account auth

0.26.0 / 2020-02-25
===================
  * Fix a large percentage of EOFs from watches [#146](https://github.com/kube-rs/kube-rs/issues/146)
  * => default timeout down to 290s from 300s
  * => `Reflector` now re-lists a lot less [#146](https://github.com/kube-rs/kube-rs/issues/146)
  * Fix decoder panic with async-compression (probably) [#144](https://github.com/kube-rs/kube-rs/issues/144)
  * `Informer::poll` can now be used with `TryStream`
  * Exposed `Config::read` and `Config::read_from` - [#124](https://github.com/kube-rs/kube-rs/issues/124)
  * Fix typo on `Api::StatefulSet`
  * Fix typo on `Api::Endpoints`
  * Add `Api::v1CustomResourceDefinition` when on k8s >= 1.17
  * Renamed `Void` to `NotUsed`

0.25.0 / 2020-02-09
===================
  * initial rustls support [#114](https://github.com/kube-rs/kube-rs/pull/114) (some local kube config issues know [#120](https://github.com/kube-rs/kube-rs/issues/120))
  * crate does better version checking against openapi features - [#106](https://github.com/kube-rs/kube-rs/issues/106)
  * initial `log_stream` support - [#109](https://github.com/kube-rs/kube-rs/issues/109)

0.24.0 / 2020-01-26
===================
  * Add support for ServiceAccount, Role, ClusterRole, RoleBinding, Endpoint - [#113](https://github.com/kube-rs/kube-rs/pull/113) + [#111](https://github.com/kube-rs/kube-rs/pull/111)
  * Upgrade k8s-openapi to 0.7 => breaking changes: https://github.com/Arnavion/k8s-openapi/blob/master/CHANGELOG.md#v070-2020-01-23

0.23.0 / 2019-12-31
===================
  * Bump tokio and reqwest to 0.2 and 0.10
  * Fix bug in `log` fetcher - [#107](https://github.com/kube-rs/kube-rs/pull/107)
  * Temporarily allow invalid certs when testing on macosx - [#105](https://github.com/kube-rs/kube-rs/pull/105)

0.22.2 / 2019-12-04
===================
  * Allow sharing Reflectors between threads - [#97](https://github.com/kube-rs/kube-rs/issues/97)
  * Fix Reflector pararall lock issue (`poll` no longer blocks `state`)

0.22.1 / 2019-11-30
===================
  * Improve Reflector reset algorithm (clear history less)

0.22.0 / 2019-11-29
===================
  * Default watch timeouts changed to 300s everywhere
  * This increases efficiency of Informers and Reflectors by keeping the connection open longer.
  * However, if your Reflector relies on frequent polling you can set `timeout` or hide the `poll()` in a different context so it doesn't block your main work
  * Internal `RwLock` changed to a `futures::Mutex` for soundness / proper non-blocking - [#94](https://github.com/kube-rs/kube-rs/issues/94)
  * blocking `Reflector::read()` renamed to `async Reflector::state()`
  * Expose `metadata.creation_timestamp` and `.deletion_timestamp` (behind openapi flag) - [#93](https://github.com/kube-rs/kube-rs/issues/93)

0.21.0 / 2019-11-29
===================
  * All watch calls returns a stream of `WatchEvent` - [#92](https://github.com/kube-rs/kube-rs/pull/92)
  * `Informer::poll` now returns a stream - [#92](https://github.com/kube-rs/kube-rs/pull/92)

0.20.1 / 2019-11-21
===================
  * ObjectList now implements Iterator - [#91](https://github.com/kube-rs/kube-rs/pull/91)
  * openapi feature no longer accidentally hardcoded to v1.15 feature - [#90](https://github.com/kube-rs/kube-rs/issues/90)

0.19.0 / 2019-11-15
==================
  * kube::Error is now a proper error enum and not a Fail impl (thiserror)
  * soft-tokio dependency removed for futures-timer
  * gzip re-introduced

0.18.1 / 2019-11-11
==================
  * Fix unpinned gzip dependency breakage - [#87](https://github.com/kube-rs/kube-rs/issues/87)

0.18.0 / 2019-11-07
==================
  * api converted to use async/await with 1.39.0 (primitively)
  * hyper upgraded to 0.10-alpha
  * synchronous sleep replaced with tokio timer
  * `Log` trait removed in favour of internal marker trait

0.17.0 / 2019-10-22
==================
  * Add support for oidc providerss with `auth-provider` w/o `access-token` - [#70](https://github.com/kube-rs/kube-rs/pull/70)
  * Bump most dependencies to more recent versions
  * Expose custom client creation
  * Added support for `v1beta1Ingress`
  * Expose incluster_config::load_default_ns - [#74](https://github.com/kube-rs/kube-rs/pull/74)

0.16.1 / 2019-08-09
==================
  * Add missing `uid` field on `ObjectMeta::ownerReferences`

0.16.0 / 2019-08-09
==================
  * Add `Reflector::get` and `Reflector::get_within` as cheaper getters
  * Add support for OpenShift kube configs with multiple CAs - via [#64](https://github.com/kube-rs/kube-rs/pull/64)
  * Add missing `ObjectMeta::ownerReferences`
  * Reduced memory consumption during compile with `k8s-openapi@0.5.1` - [#62](https://github.com/kube-rs/kube-rs/issues/62)

0.15.1 / 2019-08-18
==================
  * Fix compile issue on `1.37.0` with `Utc` serialization
  * Fix `Void` not having `Serialize` derive

0.15.0 / 2019-08-11
==================
  * Added support for `v1Job` resources - via [#58](https://github.com/kube-rs/kube-rs/pull/58)
  * Added support for `v1Namespace`, `v1DaemonSet`, `v1ReplicaSet`, `v1PersistentVolumeClaim`, `v1PersistentVolume`, `v1ResourceQuota`, `v1HorizontalPodAutoscaler` - via [#59](https://github.com/kube-rs/kube-rs/pull/59)
  * Added support for `v1beta1CronJob`, `v1ReplicationController`, `v1VolumeAttachment`, `v1NetworkPolicy` - via [#60](https://github.com/kube-rs/kube-rs/issues/60)
  * `k8s-openapi` optional dependency bumped to `0.5.0` (for kube 1.14 structs)

0.14.0 / 2019-08-03
==================
  * `Reflector::read` now returns a `Vec<K>`` rather than a `Vec<(name, K)>`:
    This fixes an unsoundness bug internally - [#56](https://github.com/kube-rs/kube-rs/pull/56) via @gnieto

0.13.0 / 2019-07-22
==================
  * Experimental oauth2 support for some providers - via [#44](https://github.com/kube-rs/kube-rs/issues/44) :
    - a big cherry-pick from various prs upstream originally for GCP
    - EKS works with setup in https://github.com/kube-rs/kube-rs/pull/20#issuecomment-511767551

0.12.0 / 2019-07-18
==================
  * Added support for `Log` subresource - via [#50](https://github.com/kube-rs/kube-rs/pull/50)
  * Added support for `v1ConfigMap` with example - via [#49](https://github.com/kube-rs/kube-rs/pull/49)
  * Demoted some spammy info messages from Reflector

0.11.0 / 2019-07-10
==================
  * Added `PatchParams` with `PatchStrategy` to allow arbitrary patch types - [#24](https://github.com/kube-rs/kube-rs/issues/24) via @ragne
  * `Event` renamed to `v1Event` to match non-slowflake type names
  * `v1Service` support added
  * Added `v1Secret` snowflake type and a `secret_reflector` example

0.10.0 / 2019-06-03
==================
  * `Api<P, U>` is now `Api<K>` for some `KubeObject` K:
    - Big change to allow snowflake objects ([#35](https://github.com/kube-rs/kube-rs/issues/35)) - but also slightly nicer
    - You want aliases `type Pod = Object<PodSpec, PodStatus>`
    - This gives you the required `KubeObject` trait impl for free
  * Added `Event` native type to prove snowflakes can be handled - [#35](https://github.com/kube-rs/kube-rs/issues/35)

  * `ApiStatus` renamed to `Status` to match kube api conventions [#36](https://github.com/kube-rs/kube-rs/issues/36)
  * Rename `Metadata` to `ObjectMeta` [#36](https://github.com/kube-rs/kube-rs/issues/36)
  * Added `ListMeta` for `ObjectList` and `Status` [#36](https://github.com/kube-rs/kube-rs/issues/36)
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
  * Revert `client.request` return type change (back to response only from pre-0.7.0 [#28](https://github.com/kube-rs/kube-rs/issues/28))
  * `delete` now returns `Either<Object<P, U>, ApiStatus> - for bug[#32](https://github.com/kube-rs/kube-rs/issues/32)
  * `delete_collection` now returns `Either<ObjectList<Object<P, U>>, ApiStatus> - for bug[#32](https://github.com/kube-rs/kube-rs/issues/32)
  * `Informer::new` renamed to `Informer::raw`
  * `Reflector::new` renamed to `Reflector::raw`
  * `Reflector::new` + `Informer::new` added for "openapi" compile time feature (does not require specifying the generic types)

0.7.0 / 2019-05-27
==================
  * Expose list/watch parameters [#11](https://github.com/kube-rs/kube-rs/issues/11)
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
  * Reflectors no longer cache `events` - see [#6](https://github.com/kube-rs/kube-rs/issues/6)

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
