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
cargo run --example dynamic_api
NAMESPACE=dev cargo run --example log_stream -- kafka-manager-7d4f4bd8dc-f6c44
```

## kube-derive focused examples
How deriving `CustomResource` works in practice, and how it interacts with the [schemars](https://github.com/GREsau/schemars/) dependency.

```sh
cargo run --example crd_derive
cargo run --example crd_derive_schema
cargo run --example crd_derive_no_schema --no-default-features --features=native-tls
```

The last one opts out from the default `schema` feature from `kube-derive` (and thus the need for you to derive/impl `JsonSchema`).

**However**: without the `schema` feature, it's left **up to you to fill in a valid openapi v3 schema**, as schemas are **required** for [v1::CustomResourceDefinitions](https://docs.rs/k8s-openapi/0.10.0/k8s_openapi/apiextensions_apiserver/pkg/apis/apiextensions/v1/struct.CustomResourceDefinition.html), and the generated crd will be rejected by the apiserver if it's missing. As the last example shows, you can do this directly without `schemars`.

Note that these examples also contain tests for CI, and are invoked with the same parameters, but using `cargo test` rather than `cargo run`.

## kube-runtime focused examples

### watchers
These example watch a single resource and does some basic filtering on the watchevent stream:

```sh
# watch all configmap events in a namespace
NAMESPACE=dev cargo run --example configmap_watcher
# watch unready pods in a namespace
NAMESPACE=dev cargo run --example pod_watcher
# watch all event events
cargo run --example event_watcher
# watch deployments, configmaps, secrets in one namespace
NAMESPACE=dev cargo run --example multi_watcher
# watch broken nodes and cross reference with events api
cargo run --example node_watcher
# watch arbitrary, untyped objects across all namespaces
cargo run --example dynamic_watcher
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

For the [`crd_reflector`](crd_reflector.rs) you need to create the `Foo` CRD first:

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
