## Examples of how to use kube

This directory contains a number of examples showcasing various capabilities of
the `kube` crates.

All examples can be executed with:

```
cargo run --example $name
```

Examples in general show a common flows. These all have logging of this library set up to `debug`, and frequently pick up on the `NAMESPACE` evar.

## kube focused api examples
For a basic overview of how to use the `Api` try:

```sh
cargo run --example crd_api
cargo run --example job_api
cargo run --example log_stream
cargo run --example pod_api
NAMESPACE=dev cargo run --example log_stream -- kafka-manager-7d4f4bd8dc-f6c44
```

## kube-runtime focused examples

### watchers
These example watch a single resource and does some basic filtering on the watchevent stream:

```sh
# watch all configmap events in a namespace
cargo run --example configmap_watcher
# watch unready pods in a namespace
NAMESPACE=dev cargo run --example pod_watcher
# watch all event events
cargo run --example event_watcher
# watch broken nodes and cross reference with events api
cargo run --example node_watcher
```

### controllers
Requires you creating the custom resource first:

```sh
kubectl apply -f configmapgen_controller_crd.yaml
cargo run --example configmapgen_controller &
kubectl apply -f configmapgen_controller_object.yaml
```

### reflectors
These examples watch resources as well as give a store access point:

```sh
# Watch namespace pods and print the current pod count every event
cargo run --example pod_reflector
# Watch nodes for applied events and current active nodes
cargo run --example node_reflector
# Watch namespace deployments for applied events and current deployments
cargo run --example deployment_reflector
# Watch namespaced secrets for applied events and print secret keys in a task
cargo run --example secret_reflector
# Watch namespaced configmaps for applied events and print store info in task
cargo run --example configmap_reflector
# Watch namespaced foo crs for applied events and print store info in task
cargo run --example crd_reflector
```

For the [`crd_reflector](crd_reflector.rs) you need to create the `Foo` CRD first:

```sh
kubectl apply -f foo.yaml
cargo run --example crd_reflector
```

then you can `kubectl apply -f crd-baz.yaml`, or `kubectl delete -f crd-baz.yaml -n default`, or `kubectl edit foos baz -n default` to verify that the events are being picked up.

## rustls
Disable default features and enable `rustls-tls`:

```sh
cargo run --example pod_watcher --no-default-features --features=rustls-tls
```
