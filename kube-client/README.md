# kube-client
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Silver-blue.svg?style=plastic&colorB=C0C0C0&colorA=306CE8)](https://github.com/kubernetes/design-proposals-archive/blob/main/api-machinery/csi-new-client-library-procedure.md#client-capabilities)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](https://github.com/kubernetes/design-proposals-archive/blob/main/api-machinery/csi-new-client-library-procedure.md#client-support-level)

The rust counterpart to [kubernetes/client-go](https://github.com/kubernetes/apimachinery).
Contains the IO layer plus the core Api layer, and also as well as config parsing.

## Usage
This crate, and all its features, are re-exported from the facade-crate `kube`.

## Docs
See the **[kube-client API Docs](https://docs.rs/kube-client/)**

## Development
Help very welcome! To help out on this crate check out these labels:
- https://github.com/kube-rs/kube/labels/client
- https://github.com/kube-rs/kube/labels/api
- https://github.com/kube-rs/kube/labels/config
