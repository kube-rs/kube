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
    println!("Kind {}", Foo::KIND);
    let mut foo = Foo::new("hi", MyFoo {
        name: "hi".into(),
        info: None,
    });
    foo.status = Some(FooStatus { is_bad: true });
    println!("Spec: {:?}", foo.spec);
    let crd = serde_json::to_string_pretty(&Foo::crd()).unwrap();
    println!("Foo CRD: \n{}", crd);
}

// some tests
// Verify Foo::crd
#[test]
fn verify_crd() {
    let crd = Foo::crd();
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
                "title": "Foo",
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
    let outputcrd = serde_json::from_value(output).expect("expected output is valid");
    assert_eq!(crd, outputcrd);
}

#[test]
fn verify_resource() {
    use static_assertions::{assert_impl_all, assert_impl_one};
    assert_eq!(Foo::KIND, "Foo");
    assert_eq!(Foo::GROUP, "clux.dev");
    assert_eq!(Foo::VERSION, "v1");
    assert_eq!(Foo::API_VERSION, "clux.dev/v1");
    assert_impl_all!(Foo: k8s_openapi::Resource, k8s_openapi::Metadata, Default);
    assert_impl_one!(MyFoo: JsonSchema);
}

#[test]
fn verify_default() {
    let fdef = Foo::default();
    let ser = serde_yaml::to_string(&fdef).unwrap();
    let exp = r#"---
apiVersion: clux.dev/v1
kind: Foo
metadata: {}
spec:
  name: """#;
    assert_eq!(exp, ser);
}
