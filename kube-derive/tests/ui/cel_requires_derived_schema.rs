#![allow(missing_docs)]
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// `#[kube(cel)]` generates `validate_cel` from the derived schema, so it cannot be combined
// with `schema = "manual"` (a manual schema is not forced through `KubeSchema` and would carry
// no `x-kubernetes-validations`, making the generated method validate nothing).
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", schema = "manual", cel)]
// Implement `JsonSchema` for `Foo` so only the error above is reported; the follow-up
// E0277 points into schemars and its rendering changes between rustc versions.
#[kube(derive = "JsonSchema")]
struct FooSpec {
    foo: String,
}

fn main() {}
