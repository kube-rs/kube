#![allow(missing_docs)]

use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", no_spec)]
enum FooFields {
    A,
    B,
}

fn main() {}
