# kube-rs
[![CircleCI](https://circleci.com/gh/clux/kube-rs.svg?style=shield)](https://circleci.com/gh/clux/kube-rs)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-alpha-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)
[![Crates.io](https://img.shields.io/crates/v/kube.svg)](https://crates.io/crates/kube)
[![Discord chat](https://img.shields.io/discord/500028886025895936.svg?logo=discord&style=plastic)](https://discord.gg/tokio)

Rust client for [Kubernetes](http://kubernetes.io) in the style of a more generic [client-go](https://github.com/kubernetes/client-go). It makes certain assumptions about the kubernetes api to allow writing generic abstractions, and as such contains rust reinterpretations of `Reflector` and `Informer` to allow writing kubernetes controllers/watchers/operators more easily.

NB: This library is currently undergoing a lot of changes with async/await stabilizing. Please check the [CHANGELOG](./CHANGELOG.md) when upgrading.

## Installation
Select a version of `kube` along with the [generated k8s api types](https://github.com/Arnavion/k8s-openapi) that corresponds to your cluster version:

```toml
[dependencies]
kube = "0.28.0"
k8s-openapi = { version = "0.7.1", default-features = false, features = ["v1_15"] }
```

Note that turning off `default-features` for `k8s-openapi` is recommended to speed up your compilation (and we provide an api anyway).

## Usage
See the [examples directory](./kube/examples) for how to watch over resources in a simplistic way.

**[API Docs](https://docs.rs/kube/)**

See [version-rs](https://github.com/clux/version-rs) for a super light (~100 lines), actix*, prometheus, deployment api setup.

See [controller-rs](https://github.com/clux/controller-rs) for a full actix* example, with circleci, and kube yaml.

NB: actix examples with futures are due for a rewrite with the current version.

## Api
The direct `Api` type takes a client, and is constructed with either the `::global` or `::namespaced` functions:

```rust
use k8s_openapi::api::core::v1::Pod;
let pods: Api<Pod> = Api::namespaced(client, "default");

let p = pods.get("blog").await?;
println!("Got blog pod with containers: {:?}", p.spec.unwrap().containers);

let patch = json!({"spec": {
    "activeDeadlineSeconds": 5
}});
let patched = pods.patch("blog", &pp, serde_json::to_vec(&patch)?).await?;
assert_eq!(patched.spec.active_deadline_seconds, Some(5));

pods.delete("blog", &DeleteParams::default()).await?;
```

See the examples ending in `_api` examples for more detail.

## Custom Resource Definitions
Working with custom resources uses automatic code-generation via [proc_macros in kube-derive](./kube-derive). You only need to `#[derive(CustomResource)]` on a struct:

```rust
#[derive(CustomResource, Serialize, Deserialize, Default, Clone)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
pub struct FooSpec {
    name: String,
    info: String,
}
```

Then you can use a lot of generated code as:

```rust
fn main() {
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    println!("kind = {}", Foo::KIND); // from k8s_openapi::Resource
    let foos: Api<Foo> = Api::namespaced(client, "default");
    let f = Foo {
        metadata: ObjectMeta::default(),
        spec: FooSpec::default()
    };
    // Print CRD
    println!("{}", serde_json::to_string_pretty(&foo.crd());
}
```

There are a ton of kubebuilder like instructions that you can annotate with here. See the `crd_` prefixed [examples](./kube/examples) for more.

## Runtime
The optional `kube::runtime` module contains sets of higher level abstractions on top of the `Api` and `Resource` types so that you don't have to do all the watch book-keeping yourself.

### Informer
A basic event watcher that presents a stream of api events on a resource with a given set of `ListParams`. Events are received as a raw `WatchEvent` type.

An Informer updates the last received `resourceVersion` internally on every event, before shipping the event to the app. If your controller restarts, you will receive one event for every active object at startup, before entering a normal watch.

```rust
let r = Resource::all::<Pod>();
let inf = Informer::new(client, r);
```

The main feature of `Informer<K>` is being able to subscribe to events while having a streaming `.poll()` open:

```rust
let pods = inf.poll().await?.boxed(); // starts a watch and returns a stream

while let Some(event) = pods.try_next().await? { // await next event
    handle(event).await?; // pass the WatchEvent to a handler
}
```

How you handle them is up to you, you could build your own state, you can use the `Api`, or just print events. In this example you get complete [Pod objects](https://arnavion.github.io/k8s-openapi/v0.7.x/k8s_openapi/api/core/v1/struct.Pod.html):

```rust
async fn handle(event: WatchEvent<Pod>) -> anyhow::Result<()> {
    match event {
        WatchEvent::Added(o) => {
            let containers = o.spec.unwrap().containers.into_iter().map(|c| c.name).collect::<Vec<_>>();
            println!("Added Pod: {} (containers={:?})", Meta::name(&o), containers);
        },
        WatchEvent::Modified(o) => {
            let phase = o.status.unwrap().phase.unwrap();
            println!("Modified Pod: {} (phase={})", Meta::name(&o), phase);
        },
        WatchEvent::Deleted(o) => {
            println!("Deleted Pod: {}", Meta::name(&o));
        },
        WatchEvent::Error(e) => {
            println!("Error event: {:?}", e);
        }
    }
    Ok(())
}
```

The [node_informer example](./kube/examples/node_informer.rs) has an example of using api calls from within event handlers.

## Reflector
A cache for `K` that keeps itself up to date. It does not expose events, but you can inspect the state map at any time.


```rust
let r = Resource::namespaced::<Node>(&namespace);
let lp = ListParams::default()
    .labels("beta.kubernetes.io/instance-type=m4.2xlarge");
let rf = Reflector::new(client, lp, r);
```

then you should `poll()` the reflector, and `state()` to get the current cached state:

```rust
rf.poll().await?; // watches + updates state

// Clone state and do something with it
rf.state().await.into_iter().for_each(|(node)| {
    println!("Found Node {:?}", node);
});
```

Note that `poll` holds the future for [290s by default](https://github.com/kubernetes/kubernetes/issues/6513), but you can (and should) get `.state()` from another async context (see reflector examples for how to spawn an async task to do this). See also the [self-driving issue](https://github.com/clux/kube-rs/issues/151).

If you need the details of just a single object, you can use the more efficient, `Reflector::get` and `Reflector::get_within`.

## Examples
Examples that show a little common flows. These all have logging of this library set up to `debug`, and where possible pick up on the `NAMSEPACE` evar.

```sh
# watch pod events
cargo run --example pod_informer
# watch event events
cargo run --example event_informer
# watch for broken nodes
cargo run --example node_informer
```

or for the reflectors:

```sh
cargo run --example pod_reflector
cargo run --example node_reflector
cargo run --example deployment_reflector
cargo run --example secret_reflector
cargo run --example configmap_reflector
```

for one based on a CRD, you need to create the CRD first:

```sh
kubectl apply -f examples/foo.yaml
cargo run --example crd_reflector
```

then you can `kubectl apply -f crd-baz.yaml -n default`, or `kubectl delete -f crd-baz.yaml -n default`, or `kubectl edit foos baz -n default` to verify that the events are being picked up.

For straight API use examples, try:

```sh
cargo run --example crd_api
cargo run --example job_api
cargo run --example log_stream
cargo run --example pod_api
NAMESPACE=dev cargo run --example log_stream -- kafka-manager-7d4f4bd8dc-f6c44
```

## Rustls
Kube has basic support for [rustls](https://github.com/ctz/rustls) as a replacement for the `openssl` dependency. To use this, turn off default features, and enable `rustls-tls`:

```sh
cargo run --example pod_informer --no-default-features --features=rustls-tls
```

or in `Cargo.toml`:

```toml
[dependencies]
kube = { version = "0.28.0", default-features = false, features = ["rustls-tls"] }
k8s-openapi = { version = "0.7.1", default-features = false, features = ["v1_15"] }
```

This will pull in the variant of `reqwest` that also uses its `rustls-tls` feature.

## License
Apache 2.0 licensed. See LICENSE for details.
