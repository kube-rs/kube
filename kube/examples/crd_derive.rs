use k8s_openapi::Resource;
use kube_derive::CustomResource;
use serde_derive::{Deserialize, Serialize};

/// Our spec for Foo
///
/// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
#[kube(status = "FooStatus")]
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
#[kube(
    printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
)]
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
    println!("Foo CRD: {:?}", Foo::crd());
}


// some tests
// Verify Foo::crd
#[test]
fn verify_crd() {
    use serde_json::{self, json};
    let crd = Foo::crd();
    let output = json!({
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
          "shortNames": [],
          "singular": "foo"
        },
        "scope": "Namespaced",
        "versions": [
          {
            "additionalPrinterColumns": [
              {
                "description": "name of foo",
                "jsonPath": ".spec.name",
                "name": "Spec",
                "type": "string"
              }
            ],
            "name": "v1",
            "served": true,
            "storage": true
          }
        ]
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
