//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl get only atm.
use anyhow::{bail, Result};
use clap::{arg, command};
use kube::{
    api::{Api, DynamicObject, ListParams, ResourceExt},
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
    Client,
};
use log::info;
use std::{collections::BTreeMap, env};

type KindMap = BTreeMap<String, (ApiResource, ApiCapabilities)>;
async fn discover_valid_inputs(client: Client) -> Result<KindMap> {
    let mut kinds = BTreeMap::new(); // valid inputs (keys are plural or kind)
    let discovery = Discovery::new(client).run().await?;
    // NB: relies on sort order of discovery groups for precedence (which is not perfect)
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            // two entries per (TODO: extend ApiResource with shortname - its in APIResource..)
            kinds
                .entry(ar.kind.to_lowercase())
                .or_insert((ar.clone(), caps.clone()));
            kinds.entry(ar.plural.to_lowercase()).or_insert((ar, caps));
        }
    }
    Ok(kinds)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 0. init
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let client = Client::try_default().await?;

    // 1. arg parsing
    let matches = command!()
        .arg(arg!(-o[output]).default_value("").possible_values(["yaml", ""]))
        .arg(arg!(-l[selector]))
        .arg(arg!(-n[namespace]))
        .arg(arg!(-A - -all))
        .arg(arg!(<verb>))
        .arg(arg!(<resource>))
        .arg(arg!([name]))
        .get_matches();
    let verb = matches.value_of("verb").unwrap().to_lowercase();
    let resource = matches.value_of("resource").unwrap().to_lowercase();
    let name = matches.value_of("name").map(|x| x.to_lowercase());
    let output = matches.value_of("output").unwrap();
    let mut lp = ListParams::default();
    if let Some(label) = matches.value_of("selector") {
        lp = lp.labels(label);
    }

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
        //log::warn!("supported verbs: {:?}", caps.operations);
        bail!("resource '{}' does not support verb '{}'", resource, verb);
    }

    // 4. create an Api based on parsed parameters
    let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
        if let Some(ns) = matches.value_of("namespace") {
            Api::namespaced_with(client.clone(), ns, &ar)
        } else if matches.is_present("all") {
            Api::all_with(client.clone(), &ar)
        } else {
            Api::default_namespaced_with(client.clone(), &ar)
        }
    } else {
        Api::all_with(client.clone(), &ar)
    };

    // 5. specialized handling for each verb (but resource agnostic)
    info!("{} {} {}", verb, resource, name.clone().unwrap_or_default());
    if verb == "get" {
        let mut result: Vec<_> = if let Some(n) = &name {
            vec![api.get(&n).await?]
        } else {
            api.list(&lp).await?.items
        };
        for x in &mut result {
            x.metadata.managed_fields = None; // hide managed fields by default
        }

        if output == "yaml" {
            println!("{}", serde_yaml::to_string(&result)?);
        } else {
            // Display style; size colums according to biggest name
            let max_name = result.iter().map(|x| x.name().len() + 2).max().unwrap_or(63);
            println!("{0:<width$} {1:<20}", "NAME", "CREATED", width = max_name);
            for inst in result {
                let name = inst.name();
                let created = inst.creation().unwrap().0;
                println!("{0:<width$} {1:<20}", name, created, width = max_name);
            }
        }
    } else if verb == "delete" {
        if let Some(n) = &name {
            api.delete(n, &Default::default()).await?;
            // TODO: await_condition is_deleted
        } else {
            api.delete_collection(&Default::default(), &lp).await?;
        }
    }

    Ok(())
}
