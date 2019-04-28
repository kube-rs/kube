# kubernetes-rust

[![Build Status](https://travis-ci.com/clux/kubernetes-rust.svg?branch=master)](https://travis-ci.com/clux/kubernetes-rust)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Bronze-blue.svg?style=plastic&colorB=cd7f32&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)

Rust client for [Kubernetes](http://kubernetes.io) API.
A temporary? fork of [ynqa/kubernetes-rust](https://github.com/ynqa/kubernetes-rust).

This version has more error handling and a `Reflector` for easy caching of CRD state.

## Example

See [operator-rs](https://github.com/clux/operator-rs)
