use kube::CustomResourceExt;
use kube_derive::CustomResource;
use schemars::{
    schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
    JsonSchema,
};
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
    fn schema_name() -> String {
        "Bar".to_string()
    }

    fn json_schema(__gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            object: Some(Box::new(ObjectValidation {
                required: ["spec".to_string()].into(),
                properties: [(
                    "spec".to_string(),
                    Schema::Object(SchemaObject {
                        instance_type: Some(InstanceType::Object.into()),
                        object: Some(Box::new(ObjectValidation {
                            required: ["bars".to_string()].into(),
                            properties: [(
                                "bars".to_string(),
                                Schema::Object(SchemaObject {
                                    instance_type: Some(InstanceType::Integer.into()),
                                    ..SchemaObject::default()
                                }),
                            )]
                            .into(),
                            ..ObjectValidation::default()
                        })),
                        ..SchemaObject::default()
                    }),
                )]
                .into(),
                ..ObjectValidation::default()
            })),
            ..SchemaObject::default()
        })
    }
}

fn main() {
    let crd = Bar::crd();
    println!("{}", serde_yaml::to_string(&crd).unwrap());
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
