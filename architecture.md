# Architecture
This document describes the high-level architecture of kube-rs.

This is intended for contributors or people interested in architecture.

## Overview
The kube-rs repository contains 5 main crates, examples and tests.

The main crate that users generally import is `kube`, and it's a straight facade crate that re-exports from the four other crates:

- `kube_core` -> re-exported as `core`
- `kube_client` -> re-exported as `api` + `client` + `config` + `discovery`
- `kube_derive` -> re-exported as `CustomResource`
- `kube_runtime` -> re-exported as `runtime`

In terms of dependencies between these 4:

- `kube_core` is used by `kube_runtime`, `kube_derive` and `kube_client`
- `kube_client` is used by `kube_runtime`
- `kube_runtime` is the highest level abstraction

The extra indirection crate `kube` is there to avoid cyclic dependencies between the client and the runtime (if the client re-exported the runtime then the two crates would be cyclically dependent).

**NB**: We refer to these crates by their `crates.io` name using underscores for separators, but the folders have dashes as separators.

When working on features/issues with `kube-rs` you will __generally__ work inside one of these crates at a time, so we will focus on these in isolation, but talk about possible overlaps at the end.

## Kubernetes Ecosystem Considerations
The Rust ecosystem does not exist in a vaccum as we take heavy inspirations from the popular Go ecosystem. In particular:

- `core` module contains invariants from [apimachinery](https://github.com/kubernetes/apimachinery) that is preseved across individual apis
- `client::Client` is a re-envisioning of a generic [client-go](https://github.com/kubernetes/client-go)
- `runtime::Controller` abstraction follows conventions in [controller-runtime](https://github.com/kubernetes-sigs/controller-runtime)
- `derive::CustomResource` derive macro for [CRDs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/) is loosely inspired by [kubebuilder's annotations](https://book.kubebuilder.io/reference/generating-crd.html)

We do occasionally diverge on matters where following the go side is worse for the rust language, but when it comes to choosing names and finding out where some modules / functionality should reside; a precedent in `client-go`, `apimachinery`, `controller-runtime` and `kubebuilder` goes a long way.

## Generated Structs
We do not maintain the kubernetes types generated from the `swagger.json` or the protos at present moment, and we do not handle client-side validation of fields relating to these types (that's left to the api-server).

We generally use k8s-openapi's Rust bindings for Kubernetes' builtin types types, see:

- [github.com:k8s-openapi](https://github.com/Arnavion/k8s-openapi/)
- [docs.rs:k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/)

We also maintain an experimental set of Protobuf bindings, see [k8s-pb](https://github.com/kazk/k8s-pb).

## Crate Overviews
### kube-core
This crate only contains types relevant to the [Kubernetes API](https://kubernetes.io/docs/concepts/overview/kubernetes-api/), abstractions analogous to what you'll find inside [apimachinery](https://github.com/kubernetes/apimachinery/tree/master/pkg), and extra Rust traits that help us with generics further down in `kube-client`.

Starting out with the basic type modules first:

- `metadata`: the various metadata types; `ObjectMeta`, `ListMeta`, `TypeMeta`
- `request` + `response` + `subresource`: a [sans-IO](https://sans-io.readthedocs.io/) style http interface for the API
- `watch`: a generic enum and behaviour for the [watch api](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
- `params`: generic parameters passed to sans-IO request interface (`ListParams` etc, called `ListOptions` in apimachinery)

Then there are traits

- `crd`: a versioned `CustomResourceExt` trait for `kube-derive`
- `object` generic conveniences for iterating over typed lists of objects, and objects following spec/status conventions
- `resource`: a `Resource` trait for `kube-client`'s `Api` + a convenience `ResourceExt` trait for users

The most important export here is the `Resource` trait and its impls. It is a pretty complex trait, with an associated type called `DynamicType` (that is default empty). Every `ObjectMeta`-using type that comes from `k8s-openapi` gets a blanket impl of `Resource` so we can use them generically (in `kube_client::Api`).

Finally, there are two modules used by the higher level `discovery` module (in `kube-client`) and they have similar counterparts in [apimachinery/restmapper](https://github.com/kubernetes/apimachinery/blob/master/pkg/api/meta/restmapper.go) + [apimachinery/group_version](https://github.com/kubernetes/apimachinery/blob/master/pkg/runtime/schema/group_version.go):

- `discovery`: types returned by the discovery api; capabilities, verbs, scopes, key info
- `gvk`: partial type information to infer api types

The main type here from these two modules is `ApiResource` because it can also be used to construct a `kube_client::Api` instance without compile-time type information (both `DynamicObject` and `Object` has `Resource` impls where `DynamicType = ApiResource`).

### kube-client

#### config
Contains logic for determining the runtime environment (local [kubeconfigs](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/) or [in-cluster](https://kubernetes.io/docs/tasks/access-application-cluster/access-cluster/#accessing-the-api-from-a-pod)) so that we can construct our `Config` from either source.

- `Config` is the source-agnostic type (with all the information needed by our `Client`)
- `Kubeconfig` is for loading from `~/.kube/config` or from any number of kubeconfig like files set by `KUBECONFIG` evar.
- `Config::from_cluster_env` reads environment variables that are injected when running inside a pod

In general this module has similar functionality to the upstream [client-go/clientcmd](https://github.com/kubernetes/client-go/tree/7697067af71046b18e03dbda04e01a5bb17f9809/tools/clientcmd) module.

#### client
The `Client` is one of the most complicated parts of `kube-rs`, because it has the most generic interface. People can mock the `Client`, people can replace individual components and force inject headers, people can choose their own tls stack, and - in theory - use whatever http clients they want.

Generally, the `Client` is created from the properties of a `Config` to create a particular `hyper::Client` with a pre-configured amount of [tower::Layer](https://docs.rs/tower/*/tower/layer/trait.Layer.html)s (see `TryFrom<Config> for Client`), but users can also pass in an arbitrary `tower::Service` (to fully customise or to mock). The signature restrictions on `Client::new` is commensurately large.

The `tls` module contains the `openssl` or `rustls` interfaces to let users pick their tls stacks. The connectors created in that module is passed to `hyper::Client` based on feature selection.

The `Client` can be created from a particular type of using the properties in the `Config` to configure its layers. Some of our layers come straight from [tower-http](https://docs.rs/tower-http):

- `tower_http::DecompressionLayer` to deal with gzip compression
- `tower_http::TraceLayer` to propagate http request information onto [tracing](https://docs.rs/tracing) spans.
- `tower_http::AddAuthorizationLayer` to set bearer tokens / basic auth (when needed)

but we also have our own layers in the `middleware` module:

- `BaseUriLayer` prefixes `Config::base_url` to requests
- `RefreshTokenLayer` will refresh auth tokens in the kubeconfig periodically when they expire (by invoking the `client::auth` module)
- `AuthLayer` configures either `AddAuthorizationLayer` or our own `RefreshTokenLayer` depending on authentication method in the kubeconfig

(The `middleware` module is kept small to avoid mixing the business logic (`client::auth` openid connect oauth provider logic) with the tower layering glue.)

The exported layers and tls connectors are mainly exposed through the `config_ext` module's `ConfigExt` trait which is only implemented by `Config` (because the config has all the properties needed for this in general, and it helps minimise our api surface).

Finally, the `Client` manages other key aspects of IO the protocol such as:

- `Client::connect` performs an [HTTP Upgrade](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Upgrade) for specialised verbs
- `Client::request` handles 90% of all requests
- `Client::request_events` handles streaming `watch` eventss using `tokio_utils`'s `FramedRead` codec
- `Client::request_status` handles `Either<T, Status>` responses from kubernetes

#### api
The generic `Api` type and its methods.

Builds on top of the `Request` / `Response` interface in `kube_core` by parametrising over a generic type `K` that implement `Resource` (plus whatever else is needed).

The `Api` absorbs a `Client` on construction and is then configured with its `Scope` (through its `::namespaced` / `::default_namespaced` or `::all` constructors).

For dynamic types (`Object` and `DynamicObject`) it has slightly more complicated constructors which have the `_with` suffix.

The `core_methods` and most `subresource` methods generally follow this recipe:

- create `Request`
- store the kubernetes verb in the [`http::Extensions`] object
- call the request with the `Client` and tell it what type(s) to deserialize into

Some subresource methods (behind the `ws` feature) use the `remote_command` module's `AttachedProcess` interface expecting a duplex stream to deal with specialised websocket verbs (`exec` and `attach`) and is calling `Client::connect` first to get that stream.

#### discovery
Deals with dynamic discovery of what apis are available on the api-server.
Normally this can be used to discover custom resources, but also certain standard resources that vary between providers.

The `Discovery` client can be used to do a full recursive sweep of api-groups into all api resources (through `filter`/`exclude` -> `run`) and then the users can periodically re-`run` to keep the cache up to date (as kubernetes is being upgraded behind the scenes).

The `discovery` module also contains a way to run smaller queries through the `oneshot` module; e.g. resolving resource name when having group version kind, resolving every resource within one specific group, or even one group at a pinned version.

The equivalent Go logic is found in [client-go/discovery](https://github.com/kubernetes/client-go/blob/master/discovery/discovery_client.go)

### kube-derive
The smallest crate. A simple [derive proc_macro](https://doc.rust-lang.org/reference/procedural-macros.html) to generate Kubernetes wrapper structs and trait impls around a data struct.

Uses `darling` to parse `#[kube(attrs...)]` then uses `syn` and `quote` to produce a suitable syntax tree based on the attributes requested.

It ultimately contains a lot of ugly json coercing from attributes into serialization code, but this is code that everyone working with custom resources need.

It has hooks into `schemars` when using `JsonSchema` to ensure the correct type of [CRD schema](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#specifying-a-structural-schema) is attached to the right part of the generated custom resource definition.

### kube-runtime
The highest level crate that deals with the highest level abstractions (such as controllers/watchers/reflectors) and specific Kubernetes apis that need common care (finalisers, waiting for conditions, event publishing).

#### watcher
The `watcher` module contains state machine wrappers around `Api::watch` that will watch and auto-recover on allowable failures.
The `watcher` fn is the general purpose one that is similar to informers in Go land, and will watch a collection of objects. The `watch_object` is a specialised version of this that watches a single object.

#### reflector
The `reflector` module contains wrappers around `watcher` that will cache objects in memory.
The `reflector` fn wraps a `watcher` and a state `Store` that is updated on every event emitted by the `watcher`.

The reason for the difference between `watcher::Event` (created by `watcher`) and `kube::api::WatchEvent` (created by `Api::watch`) is that `watcher` will deals with desync errors and do a full relist whose result is then propagated as a single event, ensuring the `reflector` can do a single, atomic update to its state `Store`.

#### controller
The `controller` module contains the `Controller` type and its associated definitions.

The `Controller` is configured to watch one root object (configured via `::new`), and several owned objects (via `::owns`), and - once `::run` - it will hit a users `reconcile` function for every change to the root object or any of its child objects (and internally it will traverse up the object tree - usually through owner references - to find the affected root object).

The user is then meant to provide an idempotent `reconcile` fn, that does not know what underlying object was changed, to ensure the state configured in its crd, is what can be seen in the world.

To manage this, a vector of watchers is converted into a [set of streams](https://docs.rs/futures/0.3.17/futures/stream/struct.SelectAll.html) of the same type by mapping the watchers so they have the same output type. This is why `watches` and `owns` differ: `owns` looks up `OwnerReferences`, but `watches` need you to define the relation yourself with a `mapper`. The mappers we support are `trigger_owners`, `trigger_self`, and the custom `trigger_with`.

Once we have combined the stream of streams we essentially have a flattened super stream with events from multiple watchers that will act as our input events. With this, the `applier` can start running its fairly complex machinery:

1. new input events get sent to the `scheduler`
2. scheduled events are then passed them through a `Runner` preventing duplicate parallel requests for the same object
3. when running, we send the affected object to the users `reconciler` fn and await that future
4. a) on success, prepare the users `ReconcilerAction` (generally a slow requeue several minutes from now)
4. b) on failure, prepare a `ReconcilerAction` based on the users error policy (generally a backoff'd requeue with shorter initial delay)
5. Map resulting `ReconcilerAction`s through an ad-hoc `scheduler` channel
6. Resulting requeue requests through the channel are picked up at the top of `applier` and merged with input events in step 1.

Ideally, the process runs forever, and it minimises unnecessary reconcile calls (like users changing more than one related object while one reconcile is already happening).

#### finalizer
Contains a helper wrapper `finalizer` for a `reconcile` fn used by a `Controller` when a user is using [finalizers](https://kubernetes.io/docs/concepts/overview/working-with-objects/finalizers/) to handle garbage collection.

This lets the user focus on simply selecting the type of behaviour they would like to exhibit based on whether the object is being deleted or it's just being regularly reconciled (through enum matching on `finalizer::Event`). This lets the user elide checking for potential deletion timestamps and manage the state machinery of `metadata.finalizers` through jsonpatching.

#### wait
Contains helpers for waiting for `conditions`, or objects to be fully removed (i.e. waiting for finalizers post delete).

These build upon `watch_object` with specific mappers.

#### events
Contains an event `Recorder` ala [client-go/events](https://github.com/kubernetes/client-go/tree/master/tools/events) that controllers can hook into, to publish events related to their reconciliations.

## Crate Delineation and Overlaps
When working on the the client machinery, it's important to realise that there are effectively 5 layers involved:

1. Sans-IO request builder (in `kube_core::Request`)
2. IO (in `kube_client::Client`)
3. Typing (in `kube_client::Api`)
4. Helpers for using the API correctly (e.g.`kube_runtime::watcher`)
5. High-level abstractions for specific tasks (e.g. `kube_runtime::controller`)

At level 3, we we essentially have what the K8s team calls a basic client. As a consequence, new methods/subresources typically cross 2 crate boundaries (`kube_core`, `kube_client`), and needs to touch 3 main modules.

Similarly, there are also the traits and types that define what an api means in `kube_core` like `Resource` and `ApiResource`.
If modifying these, then changes to `kube-derive` are likely necessary, as it needs to directly implement this for users.

These types of cross-crate dependencies are why we expose `kube` as a single versioned facade crate that users can upgrade atomically (without being caught in the middle of a publish cycle). This also gives us better compatibility with `dependabot`.
