//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl {get, delete, watch} <resource> [name] (name optional) with labels and namespace selectors
use anyhow::{bail, Context, Result};
use clap::Parser;
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

#[derive(clap::Parser)]
struct Opts {
    #[clap(long, short, arg_enum, default_value_t)]
    output: OutputMode,
    #[clap(long, short = 'l')]
    selector: Option<String>,
    #[clap(long, short)]
    namespace: Option<String>,
    #[clap(long, short = 'A')]
    all: bool,
    verb: String,
    resource: String,
    name: Option<String>,
}

#[derive(clap::ArgEnum, Clone, PartialEq, Eq)]
enum OutputMode {
    Pretty,
    Yaml,
}
impl Default for OutputMode {
    fn default() -> Self {
        Self::Pretty
    }
}

fn resolve_api_resource(discovery: &Discovery, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
    // iterate through groups to find matching kind/plural names at recommended versions
    // and then take the minimal match by group.name (equivalent to sorting groups by group.name).
    // this is equivalent to kubectl's api group preference
    discovery
        .groups()
        .flat_map(|group| {
            group
                .recommended_resources()
                .into_iter()
                .map(move |res| (group, res))
        })
        .filter(|(_, (res, _))| {
            // match on both resource name and kind name
            // ideally we should allow shortname matches as well
            name.eq_ignore_ascii_case(&res.kind) || name.eq_ignore_ascii_case(&res.plural)
        })
        .min_by_key(|(group, _res)| group.name())
        .map(|(_, res)| res)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 0. init
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // 1. arg parsing
    let Opts {
        output,
        selector,
        namespace,
        all,
        verb,
        resource,
        name,
    } = Opts::parse();
    let mut lp = ListParams::default();
    if let Some(label) = selector {
        lp = lp.labels(&label);
    }

    // 2. discovery (to be able to infer apis from kind/plural only)
    let discovery = Discovery::new(client.clone()).run().await?;
    let (ar, caps) = resolve_api_resource(&discovery, &resource)
        .with_context(|| format!("resource {:?} not found in cluster", resource))?;

    // 3. capability sanity checks and verb -> cap remapping
    let cap = if verb == "get" && name.is_none() {
        "list".into()
    } else if verb == "apply" {
        "patch".into()
    } else {
        verb.clone() // normally the colloquial verb matches the capability verb
    };
    if !caps.supports_operation(&cap) {
        //tracing::warn!("supported verbs: {:?}", caps.operations);
        bail!("resource {:?} does not support verb {:?}", resource, cap);
    }

    // 4. create an Api based on parsed parameters
    let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
        if all {
            Api::all_with(client.clone(), &ar)
        } else if let Some(ns) = namespace {
            Api::namespaced_with(client.clone(), &ns, &ar)
        } else {
            Api::default_namespaced_with(client.clone(), &ar)
        }
    } else {
        Api::all_with(client.clone(), &ar)
    };

    // 5. specialized handling for each verb (but resource agnostic)
    tracing::info!(?verb, ?resource, name = ?name.clone().unwrap_or_default(), "requested objects");
    if verb == "get" {
        let mut result: Vec<_> = if let Some(n) = &name {
            vec![api.get(n).await?]
        } else {
            api.list(&lp).await?.items
        };
        for x in &mut result {
            x.metadata.managed_fields = None; // hide managed fields by default
        }

        match output {
            OutputMode::Yaml => println!("{}", serde_yaml::to_string(&result)?),
            OutputMode::Pretty => {
                // Display style; size colums according to biggest name
                let max_name = result.iter().map(|x| x.name().len() + 2).max().unwrap_or(63);
                println!("{0:<width$} {1:<20}", "NAME", "AGE", width = max_name);
                for inst in result {
                    let age = format_creation_since(inst.meta());
                    println!("{0:<width$} {1:<20}", inst.name(), age, width = max_name);
                }
            }
        }
    } else if verb == "delete" {
        if let Some(n) = &name {
            if let Either::Left(pdel) = api.delete(n, &Default::default()).await? {
                // await delete before returning
                await_condition(api, n, is_deleted(&pdel.uid().unwrap())).await?
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
