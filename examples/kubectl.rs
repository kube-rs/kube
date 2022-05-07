//! This is a simple imitation of the basic functionality of kubectl:
//! kubectl {get, delete, apply, watch, edit} <resource> [name]
//! with labels and namespace selectors supported.
use anyhow::{bail, Context, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::{
    apimachinery::pkg::apis::meta::v1::Time,
    chrono::{Duration, Utc},
};
use kube::{
    api::{Api, DynamicObject, ListParams, Patch, PatchParams, ResourceExt},
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
struct App {
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

impl App {
    async fn get(&self, api: Api<DynamicObject>, lp: ListParams) -> Result<()> {
        let mut result: Vec<_> = if let Some(n) = &self.name {
            vec![api.get(n).await?]
        } else {
            api.list(&lp).await?.items
        };
        result.iter_mut().for_each(|x| x.managed_fields_mut().clear()); // hide managed fields

        match self.output {
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
        Ok(())
    }

    async fn delete(&self, api: Api<DynamicObject>, lp: ListParams) -> Result<()> {
        if let Some(n) = &self.name {
            if let either::Either::Left(pdel) = api.delete(n, &Default::default()).await? {
                // await delete before returning
                await_condition(api, n, is_deleted(&pdel.uid().unwrap())).await?;
            }
        } else {
            api.delete_collection(&Default::default(), &lp).await?;
        }
        Ok(())
    }

    async fn watch(&self, api: Api<DynamicObject>, mut lp: ListParams) -> Result<()> {
        if let Some(n) = &self.name {
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
        Ok(())
    }

    async fn edit(&self, api: Api<DynamicObject>) -> Result<()> {
        if let Some(n) = &self.name {
            let mut orig = api.get(n).await?;
            orig.managed_fields_mut().clear(); // hide managed fields
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
        Ok(())
    }

    async fn apply(&self, client: Client, discovery: &Discovery) -> Result<()> {
        let ssapply = PatchParams::apply("kubectl-light").force();
        let pth = self.file.clone().expect("apply needs a -f file supplied");
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
                let api = dynamic_api(ar, caps, client.clone(), &self.namespace, false);
                trace!("Applying {}: \n{}", gvk.kind, serde_yaml::to_string(&obj)?);
                let data: serde_json::Value = serde_json::to_value(&obj)?;
                let r = api.patch(&name, &ssapply, &Patch::Apply(data)).await?;
                info!("applied {}:\n {}", gvk.kind, serde_yaml::to_string(&r)?);
            } else {
                warn!("Cannot apply document for unknown {:?}", gvk);
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let app: App = clap::Parser::parse();
    let client = Client::try_default().await?;

    // discovery (to be able to infer apis from kind/plural only)
    let discovery = Discovery::new(client.clone()).run().await?;

    // Defer to methods for verbs
    if let Some(resource) = &app.resource {
        // Common discovery, parameters, and api configuration for a single resource
        let (ar, caps) = resolve_api_resource(&discovery, &resource)
            .with_context(|| format!("resource {:?} not found in cluster", resource))?;
        let mut lp = ListParams::default();
        if let Some(label) = &app.selector {
            lp = lp.labels(label);
        }
        let api = dynamic_api(ar, caps, client.clone(), &app.namespace, app.all);

        tracing::info!(?app.verb, ?resource, name = ?app.name.clone().unwrap_or_default(), "requested objects");
        match app.verb.as_ref() {
            "edit" => app.edit(api).await?,
            "get" => app.get(api, lp).await?,
            "delete" => app.delete(api, lp).await?,
            "watch" => app.watch(api, lp).await?,
            x => bail!("unsupported verb {}", x),
        }
    } else if app.verb == "apply" {
        app.apply(client, &discovery).await? // multi-resource special behaviour
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
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, &ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, &ar)
    } else {
        Api::default_namespaced_with(client, &ar)
    }
}

fn format_creation_since(time: Option<Time>) -> String {
    format_duration(Utc::now().signed_duration_since(time.unwrap().0))
}
fn format_duration(dur: Duration) -> String {
    match (dur.num_days(), dur.num_hours(), dur.num_minutes()) {
        (days, _, _) if days > 0 => format!("{}d", days),
        (_, hours, _) if hours > 0 => format!("{}h", hours),
        (_, _, mins) => format!("{}m", mins),
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
