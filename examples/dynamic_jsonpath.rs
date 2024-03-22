use anyhow::{Context, Error};
use jsonpath_rust::JsonPathInst;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // Equivalent to `kubectl get pods --all-namespace \
    // -o jsonpath='{.items[*].spec.containers[*].image}'`
    let field_selector = std::env::var("FIELD_SELECTOR").unwrap_or_default();
    let jsonpath = {
        let path = std::env::var("JSONPATH").unwrap_or_else(|_| ".items[*].spec.containers[*].image".into());
        format!("${path}")
            .parse::<JsonPathInst>()
            .map_err(Error::msg)
            .with_context(|| {
                format!(
                    "Failed to parse 'JSONPATH' value as a JsonPath expression.\n
                     Got: {path}"
                )
            })?
    };

    let pods: Api<Pod> = Api::<Pod>::all(client);
    let list_params = ListParams::default().fields(&field_selector);
    let list = pods.list(&list_params).await?;

    // Use the given JSONPATH to filter the ObjectList
    let list_json = serde_json::to_value(&list)?;
    for res in jsonpath.find_slice(&list_json, Default::default()) {
        info!("\t\t {}", *res);
    }
    Ok(())
}
