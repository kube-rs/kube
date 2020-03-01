#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
use either::Either::{Left, Right};
use serde_json::json;

use kube::{
    api::{DeleteParams, ListParams, NotUsed, Object, ObjectList, PatchParams, PostParams, Resource},
    client::APIClient,
    config,
};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec, CustomResourceDefinitionStatus as CrdStatus,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FooStatus {
    is_bad: bool,
}

// shorthands
type Foo = Object<FooSpec, FooStatus>;
type FooMeta = Object<NotUsed, NotUsed>;
type FullCrd = Object<CrdSpec, CrdStatus>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // Manage the CRD
    let crds = Resource::v1beta1CustomResourceDefinition();

    // Delete any old versions of it first:
    let dp = DeleteParams::default();
    let req = crds.delete("foos.clux.dev", &dp)?;
    let _ = client.request_status::<FullCrd>(req).await.map(|res| match res {
        Left(res) => {
            info!(
                "Deleted {}: ({:?})",
                res.metadata.name,
                res.status.unwrap().conditions.unwrap().last()
            );
            // NB: PropagationPolicy::Foreground doesn't cause us to block here
            // we have to watch for it explicitly.. but this is a demo:
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        Right(status) => info!("Deleted foos.clux.dev: {:?}", status),
    });

    // Create the CRD so we can create Foos in kube
    let foocrd = json!({
        "metadata": {
            "name": "foos.clux.dev"
        },
        "spec": {
            "group": "clux.dev",
            "version": "v1",
            "scope": "Namespaced",
            "names": {
                "plural": "foos",
                "singular": "foo",
                "kind": "Foo",
            },
            "subresources": {
                "status": {}
            },
        },
    });

    info!("Creating CRD foos.clux.dev");
    let pp = PostParams::default();
    let patch_params = PatchParams::default();
    let req = crds.create(&pp, serde_json::to_vec(&foocrd)?)?;
    match client.request::<FullCrd>(req).await {
        Ok(o) => {
            info!("Created {} ({:?})", o.metadata.name, o.status);
            debug!("Created CRD: {:?}", o.spec);
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),                        // any other case is probably bad
    }

    // Manage the Foo CR
    let foos = Resource::customResource("foos")
        .version("v1")
        .group("clux.dev")
        .within(&namespace);

    // Create Foo baz
    info!("Creating Foo instance baz");
    let f1 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "baz" },
        "spec": { "name": "baz", "info": "old baz" },
    });
    let req = foos.create(&pp, serde_json::to_vec(&f1)?)?;
    let o = client.request::<FooMeta>(req).await?;
    info!("Created {}", o.metadata.name);

    // Verify we can get it
    info!("Get Foo baz");
    let f1cpy = client.request::<Foo>(foos.get("baz")?).await?;
    assert_eq!(f1cpy.spec.info, "old baz");

    // Replace its spec
    info!("Replace Foo baz");
    let foo_replace = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "baz",
            // Need to provide our last observed version:
            "resourceVersion": f1cpy.metadata.resourceVersion,
        },
        "spec": { "name": "baz", "info": "new baz" },
    });
    let req = foos.replace("baz", &pp, serde_json::to_vec(&foo_replace)?)?;
    let f1_replaced = client.request::<Foo>(req).await?;
    assert_eq!(f1_replaced.spec.name, "baz");
    assert_eq!(f1_replaced.spec.info, "new baz");
    assert!(f1_replaced.status.is_none());


    // Create Foo qux with status
    info!("Create Foo instance qux");
    let f2 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "qux" },
        "spec": FooSpec { name: "qux".into(), info: "unpatched qux".into() },
        "status": FooStatus::default(),
    });
    let req = foos.create(&pp, serde_json::to_vec(&f2)?)?;
    let o = client.request::<Foo>(req).await?;
    info!("Created {}", o.metadata.name);

    // Try all subresource operations:
    info!("Replace Status on Foo instance qux");
    let fs = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "qux",
            // Updates need to provide our last observed version:
            "resourceVersion": o.metadata.resourceVersion,
        },
        "status": FooStatus { is_bad: true }
    });
    let req = foos.replace_status("qux", &pp, serde_json::to_vec(&fs)?)?;
    let o = client.request::<Foo>(req).await?;
    info!("Replaced status {:?} for {}", o.status, o.metadata.name);
    assert!(o.status.unwrap().is_bad);

    info!("Patch Status on Foo instance qux");
    let fs = json!({
        "status": FooStatus { is_bad: false }
    });
    let req = foos.patch_status("qux", &patch_params, serde_json::to_vec(&fs)?)?;
    let o = client.request::<Foo>(req).await?;
    info!("Patched status {:?} for {}", o.status, o.metadata.name);
    assert!(!o.status.unwrap().is_bad);

    info!("Get Status on Foo instance qux");
    let req = foos.get_status("qux")?;
    let o = client.request::<Foo>(req).await?;
    info!("Got status {:?} for {}", o.status, o.metadata.name);
    assert!(!o.status.unwrap().is_bad);


    // Modify a Foo qux with a Patch
    info!("Patch Foo instance qux");
    let patch = json!({
        "spec": { "info": "patched qux" }
    });
    let req = foos.patch("qux", &patch_params, serde_json::to_vec(&patch)?)?;
    let o = client.request::<Foo>(req).await?;
    info!("Patched {} with new name: {}", o.metadata.name, o.spec.name);
    assert_eq!(o.spec.info, "patched qux");
    assert_eq!(o.spec.name, "qux"); // didn't blat existing params

    // Delete it
    client
        .request_status::<Foo>(foos.delete("baz", &dp)?)
        .await?
        .map_left(|f1del| {
            assert_eq!(f1del.spec.info, "old baz");
        });

    // Check we have one remaining instance
    let lp = ListParams::default();
    let req = foos.list(&lp)?;
    let res = client.request::<ObjectList<Foo>>(req).await?;
    assert_eq!(res.items.len(), 1);

    // Cleanup the full colleciton
    let req = foos.delete_collection(&lp)?;
    match client.request_status::<ObjectList<Foo>>(req).await? {
        Left(res) => {
            let deleted = res.into_iter().map(|i| i.metadata.name).collect::<Vec<_>>();
            info!("Deleted collection of foos: {:?}", deleted);
        }
        Right(status) => info!("Deleted collection: {:?}", status),
    }
    Ok(())
}
