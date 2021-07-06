# kube-rs
[![CI](https://github.com/clux/kube-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/clux/kube-rs/actions/workflows/ci.yml)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)
[![Crates.io](https://img.shields.io/crates/v/kube.svg)](https://crates.io/crates/kube)
[![Discord chat](https://img.shields.io/discord/500028886025895936.svg?logo=discord&style=plastic)](https://discord.gg/tokio)

Rust client for [Kubernetes](http://kubernetes.io) in the style of a more generic [client-go](https://github.com/kubernetes/client-go), a runtime abstraction inspired by [controller-runtime](https://github.com/kubernetes-sigs/controller-runtime), and a derive macro for [CRDs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/) inspired by [kubebuilder](https://book.kubebuilder.io/reference/generating-crd.html).

These crates make certain assumptions about the kubernetes [apimachinery](https://github.com/kubernetes/apimachinery/blob/master/pkg/apis/meta/v1/types.go) + [api concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/) to enable generic abstractions. These abstractions allow rust reinterpretations of reflectors, informers, controllers, and custom resource interfaces, so that you can write applications easily.

## Installation
Select a version of `kube` along with the generated [k8s-openapi](https://github.com/Arnavion/k8s-openapi) types corresponding for your cluster version:

```toml
[dependencies]
kube = "0.58.1"
kube-runtime = "0.58.1"
k8s-openapi = { version = "0.12.0", default-features = false, features = ["v1_20"] }
```

[Features are available](https://github.com/clux/kube-rs/blob/master/kube/Cargo.toml#L18).

We recommend turning off `default-features` for `k8s-openapi` to speed up your compilation.

## Upgrading
Please check the [CHANGELOG](./CHANGELOG.md) when upgrading.
All crates herein are versioned and [released](./release.toml) together to guarantee [compatibility before 1.0](https://github.com/clux/kube-rs/issues/508).

## Usage
See the [examples directory](./examples) for how to use any of these crates.

- **[kube API Docs](https://docs.rs/kube/)**
- **[kube-runtime API Docs](https://docs.rs/kube-runtime/)**

Official examples:

- [version-rs](https://github.com/clux/version-rs): super lightweight reflector deployment with actix 2 and prometheus metrics

- [controller-rs](https://github.com/clux/controller-rs): `Controller` owned by a `Manager` inside actix

Real world users:

- [krustlet](https://github.com/deislabs/krustlet) - a complete `WASM` running `kubelet`
- [stackabletech operators](https://github.com/stackabletech) - ([kafka](https://github.com/stackabletech/kafka-operator), [zookeeper](https://github.com/stackabletech/zookeeper-operator), and more)
- [kdash tui](https://github.com/kdash-rs/kdash) - terminal dashboard for kubernetes
- [logdna agent](https://github.com/logdna/logdna-agent-v2)
- [kubeapps pinniped](https://github.com/kubeapps/kubeapps/tree/master/cmd/pinniped-proxy)
- [kubectl-view-allocations](https://github.com/davidB/kubectl-view-allocations) - kubectl plugin to list resource allocations

## Api
The direct `Api` type takes a client, and is constructed with either the `::all` or `::namespaced` functions:

```rust
use k8s_openapi::api::core::v1::Pod;
let pods: Api<Pod> = Api::namespaced(client, "default");

let p = pods.get("blog").await?;
println!("Got blog pod with containers: {:?}", p.spec.unwrap().containers);

let patch = json!({"spec": {
    "activeDeadlineSeconds": 5
}});
let pp = PatchParams::apply("my_manager");
let patched = pods.patch("blog", &pp, &Patch::Apply(patch)).await?;
assert_eq!(patched.spec.active_deadline_seconds, Some(5));

pods.delete("blog", &DeleteParams::default()).await?;
```

See the examples ending in `_api` examples for more detail.

## Custom Resource Definitions
Working with custom resources uses automatic code-generation via [proc_macros in kube-derive](https://docs.rs/kube/latest/kube/derive.CustomResource.html).

You need to `#[derive(CustomResource)]` and some `#[kube(attrs..)]` on a spec struct:

```rust
#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
pub struct FooSpec {
    name: String,
    info: String,
}
```

Then you can use the generated wrapper struct `Foo` as a [`kube::Resource`](https://docs.rs/kube/*/kube/trait.Resource.html):

```rust
let foos: Api<Foo> = Api::namespaced(client, "default");
let f = Foo::new("my-foo", FooSpec::default());
println!("foo: {:?}", f);
println!("crd: {:?}", serde_yaml::to_string(&Foo::crd()));
```

There are a ton of kubebuilder-like instructions that you can annotate with here. See the [documentation](https://docs.rs/kube/latest/kube/derive.CustomResource.html) or the `crd_` prefixed [examples](./examples) for more.

**NB:** `#[derive(CustomResource)]` requires the `derive` feature enabled on `kube`.

## Runtime
The `kube_runtime` crate contains sets of higher level abstractions on top of the `Api` and `Resource` types so that you don't have to do all the `watch`/`resourceVersion`/storage book-keeping yourself.

### Watchers
A low level streaming interface (similar to informers) that presents `Applied`, `Deleted` or `Restarted` events.


```rust
let api = Api::<Pod>::namespaced(client, "default");
let watcher = watcher(api, ListParams::default());
```

This now gives a continual stream of events and you do not need to care about the watch having to restart, or connections dropping.

```rust
let mut apply_events = try_flatten_applied(watcher).boxed_local();
while let Some(event) = apply_events.try_next().await? {
    println!("Applied: {}", event.name());
}
```

NB: the plain stream items a `watcher` returns are different from `WatchEvent`. If you are following along to "see what changed", you should flatten it with one of the utilities like `try_flatten_applied` or `try_flatten_touched`.

## Reflectors
A `reflector` is a `watcher` with `Store` on `K`. It acts on all the `Event<K>` exposed by `watcher` to ensure that the state in the `Store` is as accurate as possible.

```rust
let nodes: Api<Node> = Api::namespaced(client, &namespace);
let lp = ListParams::default()
    .labels("beta.kubernetes.io/instance-type=m4.2xlarge");
let store = reflector::store::Writer::<Node>::default();
let reader = store.as_reader();
let rf = reflector(store, watcher(nodes, lp));
```

At this point you can listen to the `reflector` as if it was a `watcher`, but you can also query the `reader` at any point.

### Controllers
A `Controller` is a `reflector` along with an arbitrary number of watchers that schedule events internally to send events through a reconciler:

```rust
Controller::new(root_kind_api, ListParams::default())
    .owns(child_kind_api, ListParams::default())
    .run(reconcile, error_policy, context)
    .for_each(|res| async move {
        match res {
            Ok(o) => info!("reconciled {:?}", o),
            Err(e) => warn!("reconcile failed: {}", Report::from(e)),
        }
    })
    .await;
```

Here `reconcile` and `error_policy` refer to functions you define. The first will be called when the root or child elements change, and the second when the `reconciler` returns an `Err`.

## Rustls
Kube has basic support ([with caveats](https://github.com/clux/kube-rs/issues?q=is%3Aissue+is%3Aopen+rustls)) for [rustls](https://github.com/ctz/rustls) as a replacement for the `openssl` dependency. To use this, turn off default features, and enable `rustls-tls`:

```toml
[dependencies]
kube = { version = "0.58.1", default-features = false, features = ["client", "rustls-tls"] }
kube-runtime = { version = "0.58.1" }
k8s-openapi = { version = "0.12.0", default-features = false, features = ["v1_20"] }
```

This will pull in `rustls` and `hyper-rustls`.

## musl-libc
Kube will work with [distroless](https://github.com/clux/controller-rs/blob/master/Dockerfile), [scratch](https://github.com/constellation-rs/constellation/blob/27dc89d0d0e34896fd37d638692e7dfe60a904fc/Dockerfile), and `alpine` (it's also possible to use alpine as a builder [with some caveats](https://github.com/clux/kube-rs/issues/331#issuecomment-715962188)).

## License
Apache 2.0 licensed. See LICENSE for details.
