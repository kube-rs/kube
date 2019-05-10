# kube-rs
[![Build Status](https://travis-ci.org/clux/kube-rs.svg?branch=master)](https://travis-ci.org/clux/kube-rs)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-alpha-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)
[![Crates.io](https://img.shields.io/crates/v/kube.svg)](https://crates.io/crates/kube)

Rust client for [Kubernetes](http://kubernetes.io) with reinterpretations of the `Reflector` and `Informer` abstractions from the go client.

This client thus aims cater to the more common controller/operator case, but allows you sticking in dependencies like [k8s-openapi](https://github.com/Arnavion/k8s-openapi) for accurate struct representations.

## Usage
See the [examples directory](./examples) for how to watch over resources in a simplistic way.

See [controller-rs](https://github.com/clux/controller-rs) for a full example with [actix](https://actix.rs/).

**[API Docs](https://clux.github.io/kube-rs/kube/)**

## Reflector
The biggest abstraction exposed in this client is `Reflector<T, U>`. This is effectively a cache of a resource that's meant to "reflect the state in etcd".

It handles the api mechanics for watching kube resources, tracking resourceVersions, and maintaining an internal cache map.

To use it, you just feed in `T` as a `Spec` struct and `U` as a `Status` struct, which can be as complete or incomplete as you like. Here, using the complete structs via [k8s-openapi](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/api/core/v1/struct.PodSpec.html):

```rust
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
let resource = ResourceType::Pods(Some("kube-system".into()));
let rf : Reflector<PodSpec, PodStatus> = Reflector::new(client.clone(), resource.into())?;
```

then you can `poll()` the reflector, and `read()` to get the current cached state:

```rust
rf.poll()?; // blocks and updates state

// read state and use it:
rf.read()?.into_iter().for_each(|(name, p)| {
    println!("Found pod {} ({}) with {:?}",
        name,
        p.status.phase.unwrap(),
        p.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>(),
    );
});
```

The reflector itself is responsible for acquiring the write lock and update the state as long as you call `poll()` periodically.

### Informers
The simplest abstraction exposed from this client. This is a struct with the internal behaviour for watching kube resources, but keeps no internal state except the `resourceVersion`.

You tell it what type parameters correspond to; `T` should be a `Spec` struct, and `U` should be a `Status` struct. Again, these can be as complete or incomplete as you like. Here, using the complete structs via [k8s-openapi](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/api/core/v1/struct.PodSpec.html):

```rust
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
let resource = ResourceType::Pods(Some("kube-system".into()));
let inf : Informer<PodSpec, PodStatus> = Informer::new(client.clone(), resource.into())?;
```

The main difference with `Reflector<T, U>` is that the only exposed function is `.poll()` and it returns `WatchEvents` that you are meant to handle yourself:

```rust
let events = inf.poll()?;
reconcile(&client, events)?; // pass them on somewhere
```

How you handle them is up to you, you could build your own `Reflector`, or you can do more controllery logic. Here's how such a function would look:

```rust
fn reconcile(c: &APIClient, events: WatchEvents<PodSpec, PodStatus>) -> Result<(), failure::Error> {
    for ev in events {
        // use the kube api client here..
        match ev {
            WatchEvent::Added(o) => {
                let containers = o.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>();
                info!("Added Pod: {} (containers={:?})", o.metadata.name, containers);
            },
            WatchEvent::Modified(o) => {
                let phase = o.status.phase.unwrap();
                info!("Modified Pod: {} (phase={})", o.metadata.name, phase);
            },
            WatchEvent::Deleted(o) => {
                info!("Deleted Pod: {}", o.metadata.name);
            },
            WatchEvent::Error(e) => {
                warn!("Error event: {:?}", e);
            }
        }
    }
    Ok(())
}
```

## Examples
Examples that show a little common flows. These all have logging of this library set up to `trace`:

```sh
# watch pod events in kube-system
cargo run --example pod_informer
```

or for the reflectors:

```rust
cargo run --example pod_reflector
cargo run --example node_reflector
cargo run --example deployment_reflector
```

for one based on a CRD, you need to create the CRD first:

```sh
kubectl apply -f examples/foo.yaml
cargo run --example crd_reflector
```

then you can `kubectl apply -f crd-baz.yaml -n kube-system`, or `kubectl delete -f crd-baz.yaml -n kube-system`, or `kubectl edit foos baz -n kube-system` to verify that the events are being picked up.

## License
Apache 2.0 licensed. See LICENSE for details.
