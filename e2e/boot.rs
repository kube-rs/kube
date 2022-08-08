use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, ResourceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::all(client);
    for p in pods.list(&Default::default()).await? {
        tracing::info!("Found pod {}", p.name_any());
    }
    Ok(())
}
