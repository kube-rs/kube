use color_eyre::Result;
use futures::prelude::*;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{ListParams, ResourceExt},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());
    let api = Api::<Pod>::namespaced(client, &namespace);
    let watcher = watcher(api, ListParams::default());
    try_flatten_applied(watcher)
        .try_for_each(|p| async move {
            log::debug!("Applied: {}", p.name_unchecked());
            if let Some(unready_reason) = pod_unready(&p) {
                log::warn!("{}", unready_reason);
            }
            Ok(())
        })
        .await?;
    Ok(())
}

fn pod_unready(p: &Pod) -> Option<String> {
    let status = p.status.as_ref().unwrap();
    if let Some(conds) = &status.conditions {
        let failed = conds
            .into_iter()
            .filter(|c| c.type_ == "Ready" && c.status == "False")
            .map(|c| c.message.clone().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(",");
        if !failed.is_empty() {
            if p.metadata.labels.as_ref().unwrap().contains_key("job-name") {
                return None; // ignore job based pods, they are meant to exit 0
            }
            return Some(format!("Unready pod {}: {}", p.name_unchecked(), failed));
        }
    }
    None
}
