#[macro_use] extern crate kube_derive;
#[macro_use] extern crate serde_derive;
use k8s_openapi::Resource;
use kube::api::ObjectMeta;



#[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
#[kube(subresource_status)]
#[kube(subresource_scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
#[kube(printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooStatus {
    is_bad: bool,
}

fn main() {
    println!("Kind {}", Foo::KIND);
    let foo = Foo {
        metadata: ObjectMeta::default(),
        spec: FooSpec::default(),
        status: Some(FooStatus { is_bad: true }),
    };
    println!("Foo: {:?}", foo);
    println!("Foo CRD: {:?}", Foo::crd());
}
