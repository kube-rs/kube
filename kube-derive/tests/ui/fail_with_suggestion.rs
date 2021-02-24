use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", shortnames = "foo")]
struct FooSpec {
    foo: String,
}

fn main() {}
