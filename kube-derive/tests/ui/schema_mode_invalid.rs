#![allow(missing_docs)]
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "example.org", version = "v1", kind = "Foo", schema = "Disabled")]
struct FooSpec {
    foo: String,
}

fn main() {}
