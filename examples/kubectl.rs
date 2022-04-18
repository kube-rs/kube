//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl get only atm.
use anyhow::{bail, Result};
use clap::{arg, command};
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
    // TODO: ensure core group gets precedence (podmetrics.plural==pods, nodemetrics.plural==nodes)
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
    // 0. init
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let client = Client::try_default().await?;

    // 1. arg parsing
    let matches = command!()
        .arg(
            arg!(-o[OUTPUT])
                .required(false)
                .default_value("")
                .possible_values(["yaml", ""]),
        )
        .arg(arg!(<VERB>))
        .arg(arg!(<RESOURCE>))
        .arg(arg!([NAME]))
        .get_matches();
    let verb = matches.value_of("VERB").unwrap().to_lowercase();
    let resource = matches.value_of("RESOURCE").unwrap().to_lowercase();
    let name = matches.value_of("NAME").map(|x| x.to_lowercase());
    let output = matches.value_of("OUTPUT").unwrap();

    // 2. discovery (to be able to infer apis from kind/plural only)
    let kindmap = discover_valid_inputs(client.clone()).await?;
    let arac = kindmap.get(&resource);
    if arac.is_none() {
        bail!("resource '{}' not found in cluster", resource);
    }
    let (ar, caps) = arac.unwrap().clone();

    // 3. sanity checks
    // TODO: if verb == get, and name.is_some, then verb = list
    if !caps.supports_operation(&verb) {
        bail!("resource '{}' does not support verb '{}'", resource, verb);
    }

    // 4. create an Api based on parsed parameters
    let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
        //TODO: support -n or -A via Api::namespaced_with or Api::all_with resp
        Api::default_namespaced_with(client.clone(), &ar)
    } else {
        Api::all_with(client.clone(), &ar)
    };

    // 5. specialized handling for each verb (but resource agnostic)
    info!("{} {} {}", verb, resource, name.clone().unwrap_or_default());
    if verb == "get" {
        let result = if let Some(n) = &name {
            vec![api.get(&n).await?]
        } else {
            api.list(&Default::default()).await?.items
        };
        if output == "yaml" {
            println!("{}", serde_yaml::to_string(&result)?);
        } else {
            // Display style; size colums according to biggest name
            let max_name = result
                .iter()
                .map(|x| x.metadata.name.as_ref().unwrap().len() + 2)
                .max()
                .unwrap_or(63);
            println!("{0:<name_width$} {1:<20}", "NAME", "AGE", name_width = max_name);
            for inst in result {
                // TODO: impl ResourceExt for DynamicObject ?
                let name = inst.metadata.name.unwrap();
                let age = inst.metadata.creation_timestamp; // TODO: agify
                println!("{0:<name_width$} {1:<20?}", name, age, name_width = max_name);
            }
        }
    }

    Ok(())
}
