//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl {get, delete, watch} <resource> [name] (name optional) with labels and namespace selectors
use anyhow::{bail, Context, Result};
use clap::{arg, command};
use either::Either;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::chrono::{Duration, Utc};
use kube::{
    api::{Api, DynamicObject, ListParams, ObjectMeta, Resource, ResourceExt},
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
    runtime::{
        utils::try_flatten_applied,
        wait::{await_condition, conditions::is_deleted},
        watcher,
    },
    Client,
};
use log::info;
use std::env;

fn resolve_api_resource(discovery: &Discovery, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
    discovery
        .groups()
        .flat_map(|group| {
            group
                .recommended_resources()
                .into_iter()
                .map(move |res| (group, res))
        })
        .filter(|(_, (res, _))| {
            name.eq_ignore_ascii_case(&res.kind) || name.eq_ignore_ascii_case(&res.plural)
        })
        .min_by_key(|(group, _res)| group.name())
        .map(|(_, res)| res)
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
    let resource = matches.value_of("resource").unwrap();
    let name = matches.value_of("name").map(|x| x.to_lowercase());
    let output = matches.value_of("output").unwrap();
    let mut lp = ListParams::default();
    if let Some(label) = matches.value_of("selector") {
        lp = lp.labels(label);
    }

    // 2. discovery (to be able to infer apis from kind/plural only)
    let discovery = Discovery::new(client.clone()).run().await?;
    let (ar, caps) = resolve_api_resource(&discovery, resource)
        .with_context(|| format!("resource {resource:?} not found in cluster"))?;

    // 3. capability sanity checks and verb -> cap remapping
    let cap = if verb == "get" && name.is_none() {
        "list".into()
    } else if verb == "apply" {
        "patch".into()
    } else {
        verb.clone() // normally the colloquial verb matches the capability verb
    };
    if !caps.supports_operation(&cap) {
        //log::warn!("supported verbs: {:?}", caps.operations);
        bail!("resource '{}' does not support verb '{}'", resource, cap);
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
            println!("{0:<width$} {1:<20}", "NAME", "AGE", width = max_name);
            for inst in result {
                let age = format_creation_since(inst.meta());
                println!("{0:<width$} {1:<20}", inst.name(), age, width = max_name);
            }
        }
    } else if verb == "delete" {
        if let Some(n) = &name {
            match api.delete(n, &Default::default()).await? {
                Either::Left(pdel) => {
                    // await delete before returning
                    await_condition(api, n, is_deleted(&pdel.uid().unwrap())).await?
                }
                _ => {}
            }
        } else {
            api.delete_collection(&Default::default(), &lp).await?;
        }
    } else if verb == "watch" {
        let w = if let Some(n) = &name {
            lp = lp.fields(&format!("metadata.name={}", n));
            watcher(api, lp) // NB: keeps watching even if object dies
        } else {
            watcher(api, lp)
        };

        // present a dumb table for it for now. maybe drop the whole watch. kubectl does not do it anymore.
        let mut stream = try_flatten_applied(w).boxed();
        println!("{0:<width$} {1:<20}", "NAME", "AGE", width = 63);
        while let Some(inst) = stream.try_next().await? {
            let age = format_creation_since(inst.meta());
            println!("{0:<width$} {1:<20}", inst.name(), age, width = 63);
        }
    }

    Ok(())
}

fn format_creation_since(meta: &ObjectMeta) -> String {
    let ts = meta.creation_timestamp.clone().unwrap().0;
    let age = Utc::now().signed_duration_since(ts);
    format_duration(age)
}
fn format_duration(dur: Duration) -> String {
    let days = dur.num_days();
    let hours = dur.num_hours();
    let mins = dur.num_minutes();
    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h", hours)
    } else {
        format!("{}m", mins)
    }
}
