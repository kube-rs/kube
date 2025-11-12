#![allow(missing_docs)]

use kube_derive::CustomResource;

#[derive(CustomResource)]
#[kube(group = "clux.dev", version = "v1", kind = "FooEnum")]
#[kube(attr = "derive(Serialize)")]
struct FooSpec {
    int: u32,
}

fn main() {}
