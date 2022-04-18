//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl get/list only atm.
use anyhow::{bail, Result};
use kube::{
    api::{Api, DynamicObject, ResourceExt},
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
    Client,
};
use log::info;
use std::{collections::BTreeMap, env};

type KindMap = BTreeMap<String, (ApiResource, ApiCapabilities)>;
async fn discover_valid_inputs(client: Client) -> Result<KindMap> {
    let mut inputmap = BTreeMap::new(); // valid inputs (keys are plural or kind)

    let discovery = Discovery::new(client).run().await?;
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            // two entries per (TODO: extend ApiResource with shortname - its in APIResource..)
            inputmap
                .entry(ar.kind.to_lowercase())
                .or_insert((ar.clone(), caps.clone()));
            inputmap.entry(ar.plural.to_lowercase()).or_insert((ar, caps));
        }
    }
    Ok(inputmap)
}

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let client = Client::try_default().await?;

    let verb = env::args()
        .nth(1)
        .expect("usage: kubectl <verb> <resource>")
        .to_lowercase();
    let resource = env::args()
        .nth(2)
        .expect("usage: kubectl <verb> <resource>")
        .to_lowercase();
    let name = env::args().nth(3); // optional

    // Full discovery first to avoid having users to figure out the group param
    let kindmap = discover_valid_inputs(client.clone()).await?;
    let arac = kindmap.get(&resource);
    if arac.is_none() {
        bail!("resource '{}' not found in cluster", resource);
    }
    let (ar, caps) = arac.unwrap().clone();

    // TODO: if verb == get, and name.is_some, then verb = list
    if !caps.supports_operation(&verb) {
        bail!("resource '{}' does not support verb '{}'", resource, verb);
    }

    info!("kubectl {} {} {}", verb, resource, name.unwrap_or_default());

    // TODO: maybe create a api.infer_with(client, ar) ?
    let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
        //TODO: support -n or -A via Api::namespaced_with or Api::all_with resp
        Api::default_namespaced_with(client.clone(), &ar)
    } else {
        Api::all_with(client.clone(), &ar)
    };

    if verb == "get" {
        // TODO: size column according to longest member (63 hardcode should be 3+max(lengths))
        println!("{0:<63} {1:<20}", "NAME", "AGE");

        // TODO: impl ResourceExt for DynamicObject ?
        for inst in api.list(&Default::default()).await? {
            let name = inst.metadata.name.unwrap();
            let age = inst.metadata.creation_timestamp; // TODO: agify
            println!("{0:<63} {1:<20?}", name, age);
        }
    }

    Ok(())
}
