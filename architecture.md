# Architecture
This document describes the high-level architecture of kube-rs.

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
The rust ecosystem does not exist in a vaccum as we take heavy inspirations from the popular `go` ecosystem. In particular;

- `runtime::Controller` abstraction follows conventions in [controller-runtime](https://github.com/kubernetes-sigs/controller-runtime)
- `derive::CustomResource` derive macro for [CRDs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/) is loosely inspired by [kubebuilder's annotations](https://book.kubebuilder.io/reference/generating-crd.html)
- `core` module contains invariants from [apimachinery](https://github.com/kubernetes/apimachinery) that is preseved across individual apis
- `client::Client` is a re-envisioning of a generic [client-go](https://github.com/kubernetes/client-go)

When it comes to choosing names, and finding out where some modules / functionality should reside, a precedent in `client-go`, `apimachinery`, `controller-runtime` and `kubebuilder` can go a long way.

That said, we do diverge on matters where following the go side is worse for the rust language. As a particular example, [client-go's tools modules](https://github.com/kubernetes/client-go/tree/master/tools) are split across several crates and modules where they are needed:

  * `discovery` module replaces `client-go`'s [discovery](https://github.com/kubernetes/client-go/tree/master/discovery) sits inside `kube_client`
  * `runtime::events` module replaces `client-go`'s [events](https://github.com/kubernetes/client-go/tree/master/tools/events) and sits inside `kube_runtime`
  * our `config` module is top level and replaces their `clientmd`
  * more of [client-go]'s [tools modules](https://github.com/kubernetes/client-go/tree/master/tools) are split in more generic ways

## Unmanaged Parts
We do not maintain the kubernetes types generated from the swagger.json or the protos at present moment, and we do not handle client-side validation of fields relating to these types (that's left to the api-server).

For information and documentation of the kubernetes types in rust world see:

- [github.com:k8s-openapi](https://github.com/Arnavion/k8s-openapi/)
- [docs.rs:k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/)

For the protobuf supporting (__WORK IN PROJECT__) see [k8s-pb](https://github.com/kazk/k8s-pb).

## Crate Overviews
### kube-core
This crate only contains types relevant to the [Kubernetes API](https://kubernetes.io/docs/concepts/overview/kubernetes-api/), abstractions analogous to what you'll find inside [apimachinery](https://github.com/kubernetes/apimachinery/tree/master/pkg), and extra rust traits that help us with generics further down in `kube-client`.

Starting out with the basic type modules first:

- `metadata`: the various metadata types; `ObjectMeta`, `ListMeta`, `TypeMeta`
- `request` + `response` + `subresource`: a [sans-IO](https://sans-io.readthedocs.io/) style http interface for the API
- `watch`: a generic enum and behaviour for the [watch api](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
- `params`: generic parameters passed to sans-IO request interface (`ListParams` etc, called `ListOptions` in apimachinery)

Then there are traits

- `crd`: a versioned `CustomResourceExt` trait for `kube-derive`
- `object` generic conveniences for iterating over typed lists of objects, and objects following spec/status conventions
- `resource`: a `Resource` trait for `kube-client`'s `Api` + a convenience `ResourceExt` trait for users

The most important export here is the `Resource` trait and its impls. It a is pretty complex trait, with an associated type called `DynamicType` (that is default empty). Every `ObjectMeta`-using type that comes from `k8s-openapi` gets a blanket impl of `Resource` so we can use them generically (in `kube-client::Api`).

Finally, there are two modules used by the higher level `discovery` module (in `kube-client`) and they have similar counterparts in [apimachinery/restmapper](https://github.com/kubernetes/apimachinery/blob/master/pkg/api/meta/restmapper.go) + [apimachinery/group_version](https://github.com/kubernetes/apimachinery/blob/master/pkg/runtime/schema/group_version.go):

- `discovery`: types returned by the discovery api; capabilities, verbs, scopes, key info
- `gvk`: partial type information to infer api types

The main type here from these two modules is `ApiResource` because it can also be used to construct a `kube-client::Api` instance without compile-time type information (both `DynamicObject` and `Object` has `Resource` impls where `DynamicType = ApiResource`).

### kube-client
In order of complexity:
#### config
Contains logic for determining the runtime environment (local [kubeconfigs](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/) or [in-cluster](https://kubernetes.io/docs/tasks/access-application-cluster/access-cluster/#accessing-the-api-from-a-pod)) so that we can construct our `Config` from either source.

- `Config` is the source-agnostic type (with all the information needed by our `Client`)
- `Kubeconfig` is for loading from `~/.kube/config` or from any number of kubeconfig like files set by `KUBECONFIG` evar.
- `Config::from_cluster_env` ports [client-go/clientcmd/client_config]()

In general this module has similar functionality to the upstream [client-go/clientcmd](https://github.com/kubernetes/client-go/tree/7697067af71046b18e03dbda04e01a5bb17f9809/tools/clientcmd) module.

#### client
- TODO: tower / layers ...

#### api
The generic `Api` type and its methods.

Builds on top of the `Request` / `Response` interface in `kube_core` by parametrising over a generic type `K` that implement `Resource` (plus whatever else is needed).

Absorbs a `Client` on construction and is then configured with its `Scope` (through its `::namespaced` / `::default_namespaced` or `::all` constructors).

It has slightly more complicated interfaces for the dynamic types `Object` and `DynamicObject` which end in `_with`.


This type was the focus of the first half of our KubeCon2020 talk [The Hidden Generics in Kubernetes' API](https://www.youtube.com/watch?v=JmwnRcc2m2A).

#### discovery
https://github.com/kubernetes/client-go/blob/master/discovery/discovery_client.go



## Overlapping Concerns
TODO
