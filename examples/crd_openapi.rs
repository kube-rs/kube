#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
use serde_json::json;

use kube::{
    api::{OpenApi, PostParams, DeleteParams, ListParams},
    client::{APIClient, StatusCode},
    config,
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

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // Manage the CRD
    let crds = OpenApi::v1beta1CustomResourceDefinition(client.clone());

    // Delete any old versions of it first:
    let dp = DeleteParams::default();
    if let Ok((res, _)) = crds.delete("foos.clux.dev", &dp) {
        info!("Deleted {}: ({:?})", res.metadata.name,
            res.status.unwrap().conditions.unwrap().last());
        std::thread::sleep(std::time::Duration::from_millis(1000));
        // even PropagationPolicy::Foreground doesn't seem to block here..
    }

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
    match crds.create(&pp, serde_json::to_vec(&foocrd)?) {
        Ok((o, s)) => {
            info!("Created {} ({})", o.metadata.name, s);
            debug!("Created CRD: {:?}", o.spec);
        },
        Err(e) => {
            if let Some(ae) = e.api_error() {
                assert_eq!(ae.code, 409); // if you skipped delete, for instance
            } else {
                return Err(e.into()) // any other case is probably bad
            }
        },
    }

    // Manage the Foo CR
    let foos : OpenApi<FooSpec, FooStatus> = OpenApi::customResource(client, "foos")
        .version("v1")
        .group("clux.dev")
        .within("dev");

    // Create Foo baz
    info!("Creating Foo instance baz");
    let f1 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "baz" },
        "spec": { "name": "baz", "info": "old baz" },
    });
    let (o, c) = foos.create(&pp, serde_json::to_vec(&f1)?)?;
    assert_eq!(f1["metadata"]["name"], o.metadata.name);
    assert_eq!(c, StatusCode::CREATED);
    info!("Created {}", o.metadata.name);

    // Verify we can get it
    info!("Get Foo baz");
    let (f1cpy, _) = foos.get("baz")?;
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
    let (f1_replaced, _) = foos.replace("baz", &pp, serde_json::to_vec(&foo_replace)?)?;
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
    let (o, _) = foos.create(&pp, serde_json::to_vec(&f2)?)?;
    info!("Created {}", o.metadata.name);

    // Update status on qux - TODO: better cluster
    //if o.status.is_some() {
    //    info!("Replace Status on Foo instance qux");
    //    let fs = json!({
    //        "status": FooStatus { is_bad: true }
    //    });
    //    let (res, _) = foos.replace_status("qux", &pp, serde_json::to_vec(&fs)?)?;
    //    info!("Replaced status {:?} for {}", res.status, res.metadata.name);
    //} else {
    //    warn!("Not doing status replace - does the cluster support sub-resources?");
    //}

    // Modify a Foo qux with a Patch
    info!("Patch Foo instance qux");
    let patch = json!({
        "spec": { "info": "patched qux" }
    });
    let (o, _) = foos.patch("qux", &pp, serde_json::to_vec(&patch)?)?;
    info!("Patched {} with new name: {}", o.metadata.name, o.spec.name);
    assert_eq!(o.spec.info, "patched qux");
    assert_eq!(o.spec.name, "qux"); // didn't blat existing params

    // Check we have too instances
    let lp = ListParams::default();
    let (res, _) = foos.list(&lp)?;
    assert_eq!(res.items.len(), 2);

    // Cleanup the full colleciton
    let (res, _) = foos.delete_collection(&lp)?;
    let deleted = res.items.into_iter().map(|i| i.metadata.name).collect::<Vec<_>>();
    info!("Deleted collection of foos: {:?}", deleted);

    Ok(())
}
