# kube
[![Build Status](https://travis-ci.org/clux/kube-rs.svg?branch=master)](https://travis-ci.org/clux/kube-rs)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-alpha-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)
[![Crates.io](https://img.shields.io/crates/v/kube.svg)](https://crates.io/crates/kube)

Rust client for [Kubernetes](http://kubernetes.io) API forking [ynqa/kubernetes-rust](https://github.com/ynqa/kubernetes-rust).

This version has more error handling and a `Reflector` for easy caching of CRD state. It aims to cater to the more common operator case, but allows you sticking in dependencies like [k8s-openapi](https://github.com/Arnavion/k8s-openapi) for accurate struct representations.

## Examples
See the [examples directory](./examples) for how to watch over resources in a simplistic way.

See [operator-rs](https://github.com/clux/operator-rs) for a full example with [actix](https://actix.rs/).


## Reflector
The one exposed abstraction in this client is `Reflector<T,U>`. This is a struct with the internal behaviour for watching for events, and updating internal state.

Ideally, you just feed in `T` and `U` as some type of `Spec` struct and some type of `Status` struct.

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

### Handling Events
Event handling is also exposed via the reflector at the moment:

```rust
let events = rf.events()?;
reconcile(&client, events)?; // pass them on somewhere
```

you can use the exposed events however you wish:

```rust
fn reconcile(c: &APIClient, evs: WatchEvents<PodSpec, PodStatus>) -> Result<(), failure::Error> {
    for ev in &evs {
           match ev {
            // TODO: do other kube api calls with client here...
            WatchEvent::Added(o) => {
                println!("Handling Added in {}", o.metadata.name);
            },
            WatchEvent::Modified(o) => {
                println!("Handling Modified Pod in {}", o.metadata.name);
            },
            WatchEvent::Deleted(o) => {
                println!("Handling Deleted Pod in {}", o.metadata.name);
            },
            WatchEvent::Error(e) => {
                println!("Error event: {:?}", e); // ought to refresh here
            }
        }
    }
    Ok(())
}
```

Note that once you have called `.events()` the events are removed from the internal state.
