# kube-derive
Add `#[derive(CustomResource)]` to your custom resource struct.

## Installation
Add the `derive` feature to `kube`:

```toml
[dependencies]
kube = { version = "0.38.0", feature = ["derive"] }
```


## Usage
Add `#[derive(CustomResource)]` to your Custom Resource Spec struct.

Then specify `group`, `version`, and `kind` as `#[kube(attrs..)]` on your struct:

```rust
use kube::CustomResource;

#[derive(CustomResource, Serialize, Deserialize, Default, Clone)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
pub struct FooSpec {
    info: String,
}
```

Then you can use a lot of generated code as:

```rust
println!("kind = {}", Foo::KIND); // impl k8s_openapi::Resource
let foos: Api<Foo> = Api::namespaced(client, "default");
let f = Foo::new("hi", FooSpec {
    info: "informative info".into(),
});
println!("foo: {:?}", f)
println!("crd: {}", serde_yaml::to_string(Foo::crd());
```

## Kube attrs
You can customize a whole slew of things:

```rust
#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    namespaced,
    status = "FooStatus",
    derive = "PartialEq",
    shortname = "f",
    scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#,
    printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
)]
```

### group = "mygroup.tld"
Required api group.

### version = "v1"
Required api version.

### kind = "Kind"
Override the default kind name for the generated CR. You need to specify this if your struct does not end in `Spec`. If your struct is `FooSpec`, then we will infer `Foo` as the `kind`.

### namespaced
To specify that this is a namespaced resource rather than cluster level.

### derive = "Trait"
Adding `#[kube(derive = "PartialEq")]` is required if you want your generated top level type to be able to `#[derive(PartialEq)]`

### status = "StatusStructName"
Adds a status struct to the top level generated type and enables the status subresource in your crd.

### scale = r#"json"#
Allow customizing the scale struct for the [scale subresource](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#subresources).

### shortname = "sn"
Add a single shortname to the generated crd.

### printcoloum = r#"json"#
Allows adding straight json to [printcolumns](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#additional-printer-columns).

## Examples
See the `crd_` prefixed [examples](./kube/examples) for more.

## Development
Help very welcome! Kubebuilder like features, testing improvement, openapi feature. See https://github.com/clux/kube-rs/labels/derive
