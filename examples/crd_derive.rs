use k8s_openapi::Resource;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Our spec for Foo
///
/// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Default, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    struct = "FooCrd",
    namespaced,
    status = "FooStatus",
    derive = "PartialEq",
    derive = "Default",
    shortname = "f",
    scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#,
    printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
)]
pub struct MyFoo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct FooStatus {
    is_bad: bool,
}

fn main() {
    println!("Kind {}", FooCrd::KIND);
    let mut foo = FooCrd::new("hi", MyFoo {
        name: "hi".into(),
        info: None,
    });
    foo.status = Some(FooStatus { is_bad: true });
    println!("Spec: {:?}", foo.spec);
    let crd = serde_json::to_string_pretty(&FooCrd::crd()).unwrap();
    println!("Foo CRD: \n{}", crd);
}

// some tests
// Verify FooCrd::crd
#[test]
fn verify_crd() {
    let output = serde_json::json!({
      "apiVersion": "apiextensions.k8s.io/v1",
      "kind": "CustomResourceDefinition",
      "metadata": {
        "name": "foos.clux.dev"
      },
      "spec": {
        "group": "clux.dev",
        "names": {
          "kind": "Foo",
          "plural": "foos",
          "shortNames": ["f"],
          "singular": "foo"
        },
        "scope": "Namespaced",
        "versions": [
          {
            "name": "v1",
            "served": true,
            "storage": true,
            "additionalPrinterColumns": [
              {
                "description": "name of foo",
                "jsonPath": ".spec.name",
                "name": "Spec",
                "type": "string"
              }
            ],
            "schema": {
              "openAPIV3Schema": {
                "description": "Auto-generated derived type for MyFoo via `CustomResource`",
                "properties": {
                  "spec": {
                    "description": "Our spec for Foo\n\nA struct with our chosen Kind will be created for us, using the following kube attrs",
                    "properties": {
                      "info": {
                        "nullable": true,
                        "type": "string"
                      },
                      "name": {
                        "type": "string"
                      }
                    },
                    "required": [
                      "name"
                    ],
                    "type": "object"
                  },
                  "status": {
                    "nullable": true,
                    "properties": {
                      "is_bad": {
                        "type": "boolean"
                      }
                    },
                    "required": [
                      "is_bad"
                    ],
                    "type": "object"
                  }
                },
                "required": [
                  "spec"
                ],
                "title": "FooCrd",
                "type": "object"
              }
            },
            "subresources": {
              "scale": {
                "specReplicasPath": ".spec.replicas",
                "statusReplicasPath": ".status.replicas"
              },
              "status": {}
            },
          }
        ]
      }
    });
    let crd = serde_json::to_value(FooCrd::crd()).unwrap();
    assert_eq!(crd, output);
}

#[test]
fn verify_resource() {
    use static_assertions::{assert_impl_all, assert_impl_one};
    assert_eq!(FooCrd::KIND, "Foo");
    assert_eq!(FooCrd::GROUP, "clux.dev");
    assert_eq!(FooCrd::VERSION, "v1");
    assert_eq!(FooCrd::API_VERSION, "clux.dev/v1");
    assert_impl_all!(FooCrd: k8s_openapi::Resource, k8s_openapi::Metadata, Default);
    assert_impl_one!(MyFoo: JsonSchema);
}

#[test]
fn verify_default() {
    let fdef = FooCrd::default();
    let ser = serde_yaml::to_string(&fdef).unwrap();
    let exp = r#"---
apiVersion: clux.dev/v1
kind: Foo
metadata: {}
spec:
  name: """#;
    assert_eq!(exp, ser);
}
