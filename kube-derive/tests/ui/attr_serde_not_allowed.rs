#![allow(missing_docs)]

use kube_derive::CustomResource;

#[derive(CustomResource)]
#[kube(group = "clux.dev", version = "v1", kind = "FooEnum")]
#[kube(attr = "serde(rename_all=\"snake_case\")")]
struct FooSpec {
    int: u32,
}

fn main() {}
