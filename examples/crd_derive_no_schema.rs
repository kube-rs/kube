use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::{
    CustomResourceDefinition, CustomResourceValidation, JSONSchemaProps,
};
use kube_derive::CustomResource;
use serde::{Deserialize, Serialize};

/// CustomResource with manually implemented schema
///
/// Normally you would do this by deriving JsonSchema or manually implementing it / parts of it.
/// But here, we simply drop in a valid schema from a string and avoid schemars from the dependency tree entirely.
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Bar",
    namespaced,
    schema = "disabled"
)]
pub struct MyBar {
    bars: u32,
}

const MANUAL_SCHEMA: &str = r#"
type: object
properties:
  spec:
    type: object
    properties:
      bars:
        type: int
    required:
    - bars
"#;

impl Bar {
    fn crd_with_manual_schema() -> CustomResourceDefinition {
        use kube::CustomResourceExt;
        let schema: JSONSchemaProps = serde_yaml::from_str(MANUAL_SCHEMA).expect("invalid schema");

        let mut crd = <Self as CustomResourceExt>::crd();
        crd.spec.versions.iter_mut().for_each(|v| {
            v.schema = Some(CustomResourceValidation {
                open_api_v3_schema: Some(schema.clone()),
            })
        });
        crd
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let crd = Bar::crd_with_manual_schema();
    println!("{}", serde_yaml::to_string(&crd).unwrap());
}

// Verify CustomResource derivable still
#[test]
fn verify_bar_is_a_custom_resource() {
    use kube::Resource;
    use schemars::JsonSchema; // only for ensuring it's not implemented
    use static_assertions::{assert_impl_all, assert_not_impl_any};

    println!("Kind {}", Bar::kind(&()));
    let bar = Bar::new("five", MyBar { bars: 5 });
    println!("Spec: {:?}", bar.spec);
    assert_impl_all!(Bar: kube::Resource);
    assert_not_impl_any!(MyBar: JsonSchema); // but no schemars schema implemented

    let crd = Bar::crd_with_manual_schema();
    for v in crd.spec.versions {
        assert!(v.schema.unwrap().open_api_v3_schema.is_some());
    }
}
