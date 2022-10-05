use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{
        Api, ApiResource, DeleteParams, DynamicObject, GroupVersionKind, ListParams, Patch, PatchParams,
        PostParams, WatchEvent,
    },
    runtime::wait::{await_condition, conditions},
    Client, CustomResource, CustomResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// This example shows how the generated schema affects defaulting and validation.
// The integration test `crd_schema_test` in `kube-derive` contains the full CRD JSON generated from this struct.
//
// References:
// - https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#defaulting
// - https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#defaulting-and-nullable

#[derive(CustomResource, Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    namespaced,
    derive = "PartialEq",
    derive = "Default"
)]
pub struct FooSpec {
    // Non-nullable without default is required.
    //
    // There shouldn't be any ambiguity here.
    non_nullable: String,

    // Non-nullable with default value.
    //
    // Serializing will work as expected because the field cannot be `None`.
    //
    // When deserializing a response from the server, the field should always be a string because
    // the field is non-nullable and the server sets the value to the default specified in the schema.
    //
    // When deserializing some input, the default value will be set if missing.
    // However, if `null` is specified, `serde` will panic.
    // The server prunes `null` for non-nullable field since 1.20 and the default is applied.
    // To match the server's behavior exactly, we can use a custom deserializer.
    #[serde(default = "default_value")]
    non_nullable_with_default: String,

    // Nullable without default, skipping None.
    //
    // By skipping to serialize, the field won't be present in the object.
    // If serialized as `null` (next field), the object will have the field set to `null`.
    //
    // Deserializing works as expected either way. `None` if it's missing or `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    nullable_skipped: Option<String>,
    // Nullable without default, not skipping None.
    nullable: Option<String>,

    // Nullable with default, skipping None.
    //
    // By skipping to serialize when `None`, the server will set the the default value specified in the schema.
    // If serialized as `null`, the server will conserve it and the defaulting does not happen (since 1.20).
    //
    // When deserializing, the default value is used only when it's missing (`null` is `None`).
    // This is consistent with how the server handles it since 1.20.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "default_nullable")]
    nullable_skipped_with_default: Option<String>,

    // Nullable with default, not skipping None.
    //
    // The default value won't be used unless missing, so this will set the value to `null`.
    // If the resource is created with `kubectl` and if this field was missing, defaulting will happen.
    #[serde(default = "default_nullable")]
    nullable_with_default: Option<String>,

    /// Default listable field
    #[serde(default)]
    default_listable: Vec<u32>,

    // Listable field with specified 'set' merge strategy
    #[serde(default)]
    #[schemars(schema_with = "set_listable_schema")]
    set_listable: Vec<u32>,
}

// https://kubernetes.io/docs/reference/using-api/server-side-apply/#merge-strategy
fn set_listable_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    serde_json::from_value(serde_json::json!({
        "type": "array",
        "items": {
            "format": "u32",
            "minium": 0,
            "type": "integer"
        },
        "x-kubernetes-list-type": "set"
    }))
    .unwrap()
}

fn default_value() -> String {
    "default_value".into()
}

fn default_nullable() -> Option<String> {
    Some("default_nullable".into())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Show the generated CRD
    println!("Foo CRD:\n{}\n", serde_yaml::to_string(&Foo::crd())?);

    // Creating CRD v1 works as expected.
    println!("Creating CRD v1");
    let client = Client::try_default().await?;
    delete_crd(client.clone()).await?;
    assert!(create_crd(client.clone()).await.is_ok());

    // Test creating Foo resource.
    let foos = Api::<Foo>::default_namespaced(client.clone());
    // Create with defaults using typed Api first.
    // `non_nullable` and `non_nullable_with_default` are set to empty strings.
    // Nullables defaults to `None` and only sent if it's not configured to skip.
    let bar = Foo::new("bar", FooSpec { ..FooSpec::default() });
    let bar = foos.create(&PostParams::default(), &bar).await?;
    assert_eq!(bar.spec, FooSpec {
        // Nonnullable without default is required.
        non_nullable: String::default(),
        // Defaulting didn't happen because an empty string was sent.
        non_nullable_with_default: String::default(),
        // `nullable_skipped` field does not exist in the object (see below).
        nullable_skipped: None,
        // `nullable` field exists in the object (see below).
        nullable: None,
        // Defaulting happened because serialization was skipped.
        nullable_skipped_with_default: default_nullable(),
        // Defaulting did not happen because `null` was sent.
        // Deserialization does not apply the default either.
        nullable_with_default: None,
        // Empty listables to be patched in later
        default_listable: Default::default(),
        set_listable: Default::default(),
    });

    // Set up dynamic resource to test using raw values.
    let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
    let api_resource = ApiResource::from_gvk(&gvk);
    let dynapi: Api<DynamicObject> = Api::default_namespaced_with(client.clone(), &api_resource);

    // Test that skipped nullable field without default is not defined.
    let val = dynapi.get("bar").await?.data;
    println!("{:?}", val["spec"]);
    // `nullable_skipped` field does not exist, but `nullable` does.
    let spec = val["spec"].as_object().unwrap();
    assert!(!spec.contains_key("nullable_skipped"));
    assert!(spec.contains_key("nullable"));

    // Test defaulting of `non_nullable_with_default` field
    let data = DynamicObject::new("baz", &api_resource).data(serde_json::json!({
        "spec": {
            "non_nullable": "a required field",
            // `non_nullable_with_default` field is missing

            // listable values to patch later to verify merge strategies
            "default_listable": vec![2],
            "set_listable": vec![2],
        }
    }));
    let val = dynapi.create(&PostParams::default(), &data).await?.data;
    println!("{:?}", val["spec"]);
    // Defaulting happened for non-nullable field
    assert_eq!(val["spec"]["non_nullable_with_default"], default_value());

    // Listables
    assert_eq!(serde_json::to_string(&val["spec"]["default_listable"])?, "[2]");
    assert_eq!(serde_json::to_string(&val["spec"]["set_listable"])?, "[2]");

    // Missing required field (non-nullable without default) is an error
    let data = DynamicObject::new("qux", &api_resource).data(serde_json::json!({
        "spec": {}
    }));
    let res = dynapi.create(&PostParams::default(), &data).await;
    assert!(res.is_err());
    match res.err() {
        Some(kube::Error::Api(err)) => {
            assert_eq!(err.code, 422);
            assert_eq!(err.reason, "Invalid");
            assert_eq!(err.status, "Failure");
            assert_eq!(
                err.message,
                "Foo.clux.dev \"qux\" is invalid: spec.non_nullable: Required value"
            );
        }
        _ => panic!(),
    }

    // Test the manually specified merge strategy
    let ssapply = PatchParams::apply("crd_derive_schema_example").force();
    let patch = serde_json::json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "spec": {
            "default_listable": vec![3],
            "set_listable": vec![3]
        }
    });
    let pres = foos.patch("baz", &ssapply, &Patch::Apply(patch)).await?;
    assert_eq!(pres.spec.default_listable, vec![3]);
    assert_eq!(pres.spec.set_listable, vec![2, 3]);
    println!("{:?}", serde_json::to_value(pres.spec));

    delete_crd(client.clone()).await?;

    Ok(())
}

// Create CRD and wait for it to be ready.
async fn create_crd(client: Client) -> Result<CustomResourceDefinition> {
    let api = Api::<CustomResourceDefinition>::all(client);
    api.create(&PostParams::default(), &Foo::crd()).await?;

    // Wait until it's accepted and established by the api-server
    println!("Waiting for the api-server to accept the CRD");
    let establish = await_condition(api.clone(), "foos.clux.dev", conditions::is_crd_established());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), establish).await?;

    // It's served by the api - get it and return it
    let crd = api.get("foos.clux.dev").await?;
    Ok(crd)
}

// Delete the CRD if it exists and wait until it's deleted.
async fn delete_crd(client: Client) -> Result<()> {
    let api = Api::<CustomResourceDefinition>::all(client);
    if api.get("foos.clux.dev").await.is_ok() {
        api.delete("foos.clux.dev", &DeleteParams::default()).await?;

        // Wait until deleted
        let timeout_secs = 15;
        let lp = ListParams::default()
            .fields("metadata.name=foos.clux.dev")
            .timeout(timeout_secs);
        let mut stream = api.watch(&lp, "0").await?.boxed_local();
        while let Some(status) = stream.try_next().await? {
            if let WatchEvent::Deleted(_) = status {
                return Ok(());
            }
        }
        Err(anyhow!(format!("CRD not deleted after {} seconds", timeout_secs)))
    } else {
        Ok(())
    }
}
