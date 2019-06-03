#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
use either::Either::{Left, Right};
use serde_json::json;

use kube::{
    api::{Api, PostParams, DeleteParams, ListParams, Object},
    client::{APIClient},
    config,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct FooSpec {
    name: String,
    info: String,
    replicas: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FooStatus {
    is_bad: bool,
    replicas: i32,
}

type Foo = Object<FooSpec, FooStatus>;

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // Manage the CRD
    let crds = Api::v1beta1CustomResourceDefinition(client.clone());

    // Delete any old versions of it first:
    let dp = DeleteParams::default();
    // but ignore delete err if not exists
    let _ = crds.delete("foos.clux.dev", &dp).map(|res| {
        res.map_left(|o| {
            info!("Deleted {}: ({:?})", o.metadata.name,
                o.status.unwrap().conditions.unwrap().last());
            // NB: PropagationPolicy::Foreground doesn't cause us to block here
            // we have to watch for Established condition using field selector
            // but this is a demo:
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }).map_right(|s| {
            // it's gone.
            info!("Deleted foos.clux.dev: ({:?})", s);
        })
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
                "status": {},
                "scale": {
                    "specReplicasPath": ".spec.replicas",
                    "statusReplicasPath": ".status.replicas",
                }
            }
        },
    });

    info!("Creating CRD foos.clux.dev");
    let pp = PostParams::default();
    match crds.create(&pp, serde_json::to_vec(&foocrd)?) {
        Ok(o) => {
            info!("Created {} ({:?})", o.metadata.name, o.status);
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
    let foos : Api<Foo> = Api::customResource(client, "foos")
        .version("v1")
        .group("clux.dev")
        .within("default");

    // Create Foo baz
    info!("Creating Foo instance baz");
    let f1 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "baz" },
        "spec": { "name": "baz", "info": "old baz", "replicas": 1 },
    });
    let o = foos.create(&pp, serde_json::to_vec(&f1)?)?;
    assert_eq!(f1["metadata"]["name"], o.metadata.name);
    info!("Created {}", o.metadata.name);

    // Verify we can get it
    info!("Get Foo baz");
    let f1cpy = foos.get("baz")?;
    assert_eq!(f1cpy.spec.info, "old baz");

    // Replace its spec
    info!("Replace Foo baz");
    let foo_replace = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "baz",
            // Updates need to provide our last observed version:
            "resourceVersion": f1cpy.metadata.resourceVersion,
        },
        "spec": { "name": "baz", "info": "new baz", "replicas": 1 },
    });
    let f1_replaced = foos.replace("baz", &pp, serde_json::to_vec(&foo_replace)?)?;
    assert_eq!(f1_replaced.spec.name, "baz");
    assert_eq!(f1_replaced.spec.info, "new baz");
    assert!(f1_replaced.status.is_none());

    // Delete it
    foos.delete("baz", &dp)?.map_left(|f1del| {
        assert_eq!(f1del.spec.info, "old baz");
    });


    // Create Foo qux with status
    info!("Create Foo instance qux");
    let f2 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "qux" },
        "spec": FooSpec { name: "qux".into(), replicas: 0, info: "unpatched qux".into() },
        "status": FooStatus::default(),
    });
    let o = foos.create(&pp, serde_json::to_vec(&f2)?)?;
    info!("Created {}", o.metadata.name);

    // Update status on qux
    info!("Replace Status on Foo instance qux");
    let fs = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "qux",
            // Updates need to provide our last observed version:
            "resourceVersion": o.metadata.resourceVersion,
        },
        "status": FooStatus { is_bad: true, replicas: 0 }
    });
    let o = foos.replace_status("qux", &pp, serde_json::to_vec(&fs)?)?;
    info!("Replaced status {:?} for {}", o.status, o.metadata.name);
    assert!(o.status.unwrap().is_bad);

    info!("Patch Status on Foo instance qux");
    let fs = json!({
        "status": FooStatus { is_bad: false, replicas: 1 }
    });
    let o = foos.patch_status("qux", &pp, serde_json::to_vec(&fs)?)?;
    info!("Patched status {:?} for {}", o.status, o.metadata.name);
    assert!(!o.status.unwrap().is_bad);

    info!("Get Status on Foo instance qux");
    let o = foos.get_status("qux")?;
    info!("Got status {:?} for {}", o.status, o.metadata.name);
    assert!(!o.status.unwrap().is_bad);

    // Check scale subresource:
    info!("Get Scale on Foo instance qux");
    let scale = foos.get_scale("qux")?;
    info!("Got scale {:?} - {:?}", scale.spec, scale.status);
    assert_eq!(scale.status.unwrap().replicas, 1);

    // Scale up
    let fs = json!({
        "spec": { "replicas": 2 }
    });
    let o = foos.patch_scale("qux", &pp, serde_json::to_vec(&fs)?)?;
    info!("Patched scale {:?} for {}", o.spec, o.metadata.name);
    assert_eq!(o.status.unwrap().replicas, 1);
    assert_eq!(o.spec.replicas.unwrap(), 2); // we only asked for more

    // Modify a Foo qux with a Patch
    info!("Patch Foo instance qux");
    let patch = json!({
        "spec": { "info": "patched qux" }
    });
    let o = foos.patch("qux", &pp, serde_json::to_vec(&patch)?)?;
    info!("Patched {} with new name: {}", o.metadata.name, o.spec.name);
    assert_eq!(o.spec.info, "patched qux");
    assert_eq!(o.spec.name, "qux"); // didn't blat existing params

    // Check we have 1 remaining instance
    let lp = ListParams::default();
    let res = foos.list(&lp)?;
    assert_eq!(res.items.len(), 1);

    // Delete the last - expect a status back (instant delete)
    assert!(foos.delete("qux", &dp)?.is_right());

    // Cleanup the full collection - expect a wait
    match foos.delete_collection(&lp)? {
        Left(list) => {
            let deleted = list.items.into_iter().map(|i| i.metadata.name).collect::<Vec<_>>();
            info!("Deleted collection of foos: {:?}", deleted);
        },
        Right(status) => {
            info!("Deleted collection of crds: status={:?}", status);
        }
    }
    Ok(())
}
