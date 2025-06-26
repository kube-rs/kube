use std::borrow::Cow;

use kube::CustomResourceExt;
use kube_derive::CustomResource;
use schemars::{json_schema, JsonSchema};
use serde::{Deserialize, Serialize};

/// CustomResource with manually implemented `JsonSchema`
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Bar",
    namespaced,
    schema = "manual"
)]
pub struct MyBar {
    bars: u32,
}

impl JsonSchema for Bar {
    fn schema_name() -> Cow<'static, str> {
        "Bar".into()
    }

    fn json_schema(__gen: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "properties": {
                "spec": {
                    "properties": {
                        "bars": {
                            "type": "integer"
                        }
                    },
                    "required": [
                        "bars"
                    ],
                    "type": "object"
                }
            },
            "required": [
                "spec"
            ],
            "title": "Bar"
        })
    }
}

fn main() {
    let crd = Bar::crd();
    println!("{}", serde_json::to_string(&crd).unwrap());
}

// Verify CustomResource derivable still
#[test]
fn verify_bar_is_a_custom_resource() {
    use kube::Resource;
    use static_assertions::assert_impl_all;

    println!("Kind {}", Bar::kind(&()));
    let bar = Bar::new("five", MyBar { bars: 5 });
    println!("Spec: {:?}", bar.spec);
    assert_impl_all!(Bar: kube::Resource, JsonSchema);

    let crd = Bar::crd();
    for v in crd.spec.versions {
        assert!(v.schema.unwrap().open_api_v3_schema.is_some());
    }
}
