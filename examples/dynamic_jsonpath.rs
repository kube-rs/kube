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
    let jsonpath = format!(
        "{}{}",
        "$",
        std::env::var("JSONPATH").unwrap_or_else(|_| ".items[*].spec.containers[*].image".into())
    );

    let pods: Api<Pod> = Api::<Pod>::all(client);
    let list_params = ListParams::default().fields(&field_selector);
    let list = pods.list(&list_params).await?;

    // Use the given JSONPATH to filter the ObjectList
    let list_json = serde_json::to_value(&list)?;
    let res = jsonpath_lib::select(&list_json, &jsonpath).unwrap();
    info!("\t\t {:?}", res);
    Ok(())
}
