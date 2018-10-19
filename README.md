# kubernetes-rust

[![Build Status](https://travis-ci.com/ynqa/kubernetes-rust.svg?branch=master)](https://travis-ci.com/ynqa/kubernetes-rust)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Bronze-blue.svg?style=plastic&colorB=cd7f32&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)

Rust client for [Kubernetes](http://kubernetes.io) API.

## Example

List all Pods on `kube-system`:

```rust
extern crate failure;
extern crate k8s_openapi;
extern crate kubernetes;

use k8s_openapi::v1_10::api::core::v1;
use kubernetes::client::APIClient;
use kubernetes::config;

fn main() {
    let kubeconfig = config::load_kube_config().expect("failed to load kubeconfig");
    let kubeclient = APIClient::new(kubeconfig);
    let req = v1::Pod::list_core_v1_namespaced_pod(
        "kube-system",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ).expect("failed to define list pod");
    let list_pod = kubeclient
        .request::<v1::PodList>(req)
        .expect("failed to list up pods");
    println!("{:?}", list_pod);
}
```
