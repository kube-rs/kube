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
