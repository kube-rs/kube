#![allow(missing_docs)]
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// `#[kube(cel)]` generates `validate_cel` from the derived schema, so it cannot be combined
// with `schema = "manual"` (a manual schema is not forced through `KubeSchema` and would carry
// no `x-kubernetes-validations`, making the generated method validate nothing).
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", schema = "manual", cel)]
// `schema = "manual"` still needs `Foo: JsonSchema` for the generated `crd()`; derive it so the
// snapshot only holds the cel error above, not an E0277 whose rendering changes across rustc versions.
#[kube(derive = "JsonSchema")]
struct FooSpec {
    foo: String,
}

fn main() {}
