use k8s_openapi::Resource;
use kube_derive::CustomResource;
use serde::{Deserialize, Serialize};

/// Our spec for Foo
///
/// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    namespaced,
    status = "FooStatus",
    finalizer = "mygc.foos.clux.dev",
    scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#,
    printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
)]
#[kube(apiextensions = "v1beta1")] // kubernetes <= 1.16
pub struct MyFoo {
    name: String,
    info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    println!("finalizers: {:?}", foo.metadata.finalizers);
    println!("Foo CRD: \n{}", serde_json::to_string_pretty(&Foo::crd()).unwrap());
}

// some tests
// Verify Foo::crd
#[test]
fn verify_crd() {
    use serde_json::{self, json};
    let crd = Foo::crd();
    let output = json!({
      "apiVersion": "apiextensions.k8s.io/v1beta1",
      "kind": "CustomResourceDefinition",
      "metadata": {
        "name": "foos.clux.dev"
      },
      "spec": {
        "group": "clux.dev",
        "names": {
          "kind": "Foo",
          "plural": "foos",
          "shortNames": [],
          "singular": "foo"
        },
        "additionalPrinterColumns": [
          {
            "description": "name of foo",
            "JSONPath": ".spec.name",
            "name": "Spec",
            "type": "string"
          }
        ],
        "scope": "Namespaced",
        "versions": [
          {
            "name": "v1",
            "served": true,
            "storage": true
          }
        ],
        "subresources": {
          "scale": {"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"},
          "status": {}
        }
      }
    });
    let outputcrd = serde_json::from_value(output).expect("expected output is valid");
    assert_eq!(crd, outputcrd);
}

#[test]
fn verify_resource() {
    assert_eq!(Foo::KIND, "Foo");
    assert_eq!(Foo::GROUP, "clux.dev");
    assert_eq!(Foo::VERSION, "v1");
    assert_eq!(Foo::API_VERSION, "clux.dev/v1");
}
