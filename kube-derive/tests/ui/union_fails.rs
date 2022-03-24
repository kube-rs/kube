use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, JsonSchema)]
union FooSpec {
    int: u32,
}

fn main() {}
