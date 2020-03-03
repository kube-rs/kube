#[macro_use] extern crate kube_derive;
use k8s_openapi::Resource;
use kube::api::ObjectMeta;

#[derive(CustomResource, Default, Debug)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
#[kube(status)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[derive(Debug)]
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
    println!("Foo CRD: {:?}", foo.crd());
}
