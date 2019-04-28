# kubernetes-rust

[![Build Status](https://travis-ci.com/clux/kubernetes-rust.svg?branch=master)](https://travis-ci.com/clux/kubernetes-rust)
[![Client Capabilities](https://img.shields.io/badge/Kubernetes%20client-Bronze-blue.svg?style=plastic&colorB=cd7f32&colorA=306CE8)](http://bit.ly/kubernetes-client-capabilities-badge)
[![Client Support Level](https://img.shields.io/badge/kubernetes%20client-beta-green.svg?style=plastic&colorA=306CE8)](http://bit.ly/kubernetes-client-support-badge)

Rust client for [Kubernetes](http://kubernetes.io) API.
A temporary? fork of [ynqa/kubernetes-rust](https://github.com/ynqa/kubernetes-rust).

This version has more error handling and a `Reflector` for easy caching of CRD state.

## Example

See [operator-rs](https://github.com/clux/operator-rs) for ideas.

Main ideas, depending on how complicated your need is:

### Watch calls on a resource
You probably want a reflector?. See [operator-rs](https://github.com/clux/operator-rs).

### Basic calls
Use the exposed `APIClient` herein, and write Request functions directly:

```rust
pub fn list_all_crd_entries(r: &ApiResource) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = r.group, resource = r.resource, ns = r.namespace);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}
```

Then call them with:

```rust
let res = client.request::<ResourceList<Resource<T>>>(req)?;
```

where `Resource` + `ResourceList` can be constructed analogously to what's in [./src/api](https://github.com/clux/kubernetes-rust/tree/master/src/api).

You can also look at the [documentation for k8s-openapi](https://docs.rs/crate/k8s-openapi) is generating. The functions that return `http::Request` are compatible.

### Many basic calls
Pull in [k8s-openapi](https://github.com/Arnavion/k8s-openapi) and call it from the client herein. Note that not everything is available from k8s-openapi anyway. Watching Crd entries (not the core definitions) was missing in 0.4.0.
