//! This is a simple imitation of the basic functionality of kubectl
//! Supports kubectl {get, delete, apply, watch, edit} <resource> [name] (name optional) with labels and namespace selectors
use anyhow::{bail, Context, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::{
    apimachinery::pkg::apis::meta::v1::Time,
    chrono::{Duration, Utc},
};
use kube::{
    api::{Api, DynamicObject, ListParams, Patch, PatchParams, Resource, ResourceExt},
    core::GroupVersionKind,
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
    runtime::{
        utils::try_flatten_applied,
        wait::{await_condition, conditions::is_deleted},
        watcher,
    },
    Client,
};
use tracing::*;

#[derive(clap::Parser)]
struct Opts {
    #[clap(long, short, arg_enum, default_value_t)]
    output: OutputMode,
    #[clap(long, short)]
    file: Option<std::path::PathBuf>,
    #[clap(long, short = 'l')]
    selector: Option<String>,
    #[clap(long, short)]
    namespace: Option<String>,
    #[clap(long, short = 'A')]
    all: bool,
    verb: String,
    resource: Option<String>,
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
        file,
        selector,
        namespace,
        all,
        verb,
        resource,
        name,
    } = clap::Parser::parse();

    // discovery (to be able to infer apis from kind/plural only)
    let discovery = Discovery::new(client.clone()).run().await?;

    // specialized handling for apply (can handle multiple resources)
    if verb == "apply" {
        let ssapply = PatchParams::apply("kubectl-light").force();
        if let Some(pth) = file {
            let yaml =
                std::fs::read_to_string(&pth).with_context(|| format!("Failed to read {}", pth.display()))?;
            for doc in multidoc_deserialize(&yaml)? {
                let obj: DynamicObject = serde_yaml::from_value(doc)?;
                let gvk = if let Some(tm) = &obj.types {
                    GroupVersionKind::try_from(tm)?
                } else {
                    bail!("cannot apply object without valid TypeMeta {:?}", obj);
                };
                let name = obj.name();
                if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
                    let api = dynamic_api(ar, caps, client.clone(), &namespace, false);
                    trace!("Applying {}: \n{}", gvk.kind, serde_yaml::to_string(&obj)?);
                    let data: serde_json::Value = serde_json::to_value(&obj)?;
                    let r = api.patch(&name, &ssapply, &Patch::Apply(data)).await?;
                    info!("applied {:?}", r);
                } else {
                    warn!("Cannot apply document for unknown {:?}", gvk);
                }
            }
        }
    } else if let Some(resource) = resource {
        // common getters that all use the same apisesource via
        let (ar, caps) = resolve_api_resource(&discovery, &resource)
            .with_context(|| format!("resource {:?} not found in cluster", resource))?;


        let mut lp = ListParams::default();
        if let Some(label) = selector {
            lp = lp.labels(&label);
        }

        // 4. create an Api based on parsed parameters
        let api = dynamic_api(ar, caps, client.clone(), &namespace, all);

        tracing::info!(?verb, ?resource, name = ?name.clone().unwrap_or_default(), "requested objects");
        if verb == "edit" {
            if let Some(n) = &name {
                let mut orig = api.get(n).await?;
                orig.meta_mut().managed_fields = None; // hide managed fields
                let input = serde_yaml::to_string(&orig)?;
                debug!("opening {} in {:?}", orig.name(), edit::get_editor());
                let edited = edit::edit(&input)?;
                if edited != input {
                    info!("updating changed object {}", orig.name());
                    let data: DynamicObject = serde_yaml::from_str(&edited)?;
                    // NB: simplified kubectl constructs a merge-patch of differences
                    api.replace(&n, &Default::default(), &data).await?;
                }
            } else {
                warn!("need a name to edit");
            }
        } else if verb == "get" {
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
                        let age = format_creation_since(inst.creation_timestamp());
                        println!("{0:<width$} {1:<20}", inst.name(), age, width = max_name);
                    }
                }
            }
        } else if verb == "delete" {
            if let Some(n) = &name {
                if let either::Either::Left(pdel) = api.delete(n, &Default::default()).await? {
                    // await delete before returning
                    await_condition(api, n, is_deleted(&pdel.uid().unwrap())).await?
                }
            } else {
                api.delete_collection(&Default::default(), &lp).await?;
            }
        } else if verb == "watch" {
            if let Some(n) = &name {
                lp = lp.fields(&format!("metadata.name={}", n));
            }
            let w = watcher(api, lp);

            // present a dumb table for it for now. maybe drop the whole watch. kubectl does not do it anymore.
            let mut stream = try_flatten_applied(w).boxed();
            println!("{0:<width$} {1:<20}", "NAME", "AGE", width = 63);
            while let Some(inst) = stream.try_next().await? {
                let age = format_creation_since(inst.creation_timestamp());
                println!("{0:<width$} {1:<20}", inst.name(), age, width = 63);
            }
        }
    }
    Ok(())
}

fn dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    ns: &Option<String>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Namespaced {
        if all {
            Api::all_with(client, &ar)
        } else if let Some(namespace) = ns {
            Api::namespaced_with(client, namespace, &ar)
        } else {
            Api::default_namespaced_with(client, &ar)
        }
    } else {
        Api::all_with(client, &ar)
    }
}

fn format_creation_since(time: Option<Time>) -> String {
    let ts = time.unwrap().0;
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

pub fn multidoc_deserialize(data: &str) -> Result<Vec<serde_yaml::Value>> {
    use serde::Deserialize;
    let mut docs = vec![];
    for de in serde_yaml::Deserializer::from_str(data) {
        docs.push(serde_yaml::Value::deserialize(de)?);
    }
    Ok(docs)
}
