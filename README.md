# kube-rs

[![Crates.io](https://img.shields.io/crates/v/kube.svg)](https://crates.io/crates/kube)
[![Rust 1.56](https://img.shields.io/badge/MSRV-1.56-dea584.svg)](https://github.com/rust-lang/rust/releases/tag/1.56.0)
[![Discord chat](https://img.shields.io/discord/500028886025895936.svg?logo=discord&style=plastic)](https://discord.gg/tokio)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](https://github.com/kubernetes/design-proposals-archive/blob/main/api-machinery/csi-new-client-library-procedure.md#client-capabilities)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](https://github.com/kubernetes/design-proposals-archive/blob/main/api-machinery/csi-new-client-library-procedure.md#client-support-level)
[![Best Practices](https://bestpractices.coreinfrastructure.org/projects/5413/badge)](https://bestpractices.coreinfrastructure.org/projects/5413)

A [Rust](https://rust-lang.org/) client for [Kubernetes](http://kubernetes.io) in the style of a more generic [client-go](https://github.com/kubernetes/client-go), a runtime abstraction inspired by [controller-runtime](https://github.com/kubernetes-sigs/controller-runtime), and a derive macro for [CRDs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/) inspired by [kubebuilder](https://book.kubebuilder.io/reference/generating-crd.html). Hosted by [CNCF](https://cncf.io/) as a [Sandbox Project](https://www.cncf.io/sandbox-projects/)

These crates build upon Kubernetes [apimachinery](https://github.com/kubernetes/apimachinery/blob/master/pkg/apis/meta/v1/types.go) + [api concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/) to enable generic abstractions. These abstractions allow Rust reinterpretations of reflectors, controllers, and custom resource interfaces, so that you can write applications easily.

## Installation

Select a version of `kube` along with the generated [k8s-openapi](https://github.com/Arnavion/k8s-openapi) types corresponding for your cluster version:

```toml
[dependencies]
kube = { version = "0.70.0", features = ["runtime", "derive"] }
k8s-openapi = { version = "0.14.0", features = ["v1_22"] }
```

[Features are available](https://github.com/kube-rs/kube-rs/blob/master/kube/Cargo.toml#L18).

## Upgrading

Please check the [CHANGELOG](https://github.com/kube-rs/kube-rs/blob/master/CHANGELOG.md) when upgrading.
All crates herein are versioned and [released](https://github.com/kube-rs/kube-rs/blob/master/release.toml) together to guarantee [compatibility before 1.0](https://github.com/kube-rs/kube-rs/issues/508).

## Usage

See the **[examples directory](https://github.com/kube-rs/kube-rs/blob/master/examples)** for how to use any of these crates.

- **[kube API Docs](https://docs.rs/kube/)**

Official examples:

- [version-rs](https://github.com/kube-rs/version-rs): lightweight deployment `reflector` using axum
- [controller-rs](https://github.com/kube-rs/controller-rs): `Controller` of a crd inside actix

For real world projects see [ADOPTERS](https://kube.rs/adopters/).

## Api

The [`Api`](https://docs.rs/kube/*/kube/struct.Api.html) is what interacts with kubernetes resources, and is generic over [`Resource`](https://docs.rs/kube/*/kube/trait.Resource.html):

```rust
use k8s_openapi::api::core::v1::Pod;
let pods: Api<Pod> = Api::namespaced(client, "apps");

let p = pods.get("blog").await?;
println!("Got blog pod with containers: {:?}", p.spec.unwrap().containers);

let patch = json!({"spec": {
    "activeDeadlineSeconds": 5
}});
let pp = PatchParams::apply("my_controller");
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

There are a ton of kubebuilder-like instructions that you can annotate with here. See the [documentation](https://docs.rs/kube/latest/kube/derive.CustomResource.html) or the `crd_` prefixed [examples](https://github.com/kube-rs/kube-rs/blob/master/examples) for more.

**NB:** `#[derive(CustomResource)]` requires the `derive` feature enabled on `kube`.

## Runtime

The `runtime` module exports the `kube_runtime` crate and contains higher level abstractions on top of the `Api` and `Resource` types so that you don't have to do all the `watch`/`resourceVersion`/storage book-keeping yourself.

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

Kube has basic support ([with caveats](https://github.com/kube-rs/kube-rs/issues?q=is%3Aissue+is%3Aopen+rustls)) for [rustls](https://github.com/ctz/rustls) as a replacement for the `openssl` dependency. To use this, turn off default features, and enable `rustls-tls`:

```toml
[dependencies]
kube = { version = "0.70.0", default-features = false, features = ["client", "rustls-tls"] }
k8s-openapi = { version = "0.14.0", features = ["v1_22"] }
```

This will pull in `rustls` and `hyper-rustls`.

## musl-libc

Kube will work with [distroless](https://github.com/kube-rs/controller-rs/blob/master/Dockerfile), [scratch](https://github.com/constellation-rs/constellation/blob/27dc89d0d0e34896fd37d638692e7dfe60a904fc/Dockerfile), and `alpine` (it's also possible to use alpine as a builder [with some caveats](https://github.com/kube-rs/kube-rs/issues/331#issuecomment-715962188)).

## License

Apache 2.0 licensed. See LICENSE for details.
