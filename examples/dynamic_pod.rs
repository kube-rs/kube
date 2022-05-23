use kube::{
    api::{Api, ApiResource, NotUsed, Object, ResourceExt},
    Client,
};
use serde::Deserialize;
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // Here we replace heavy type k8s_openapi::api::core::v1::PodSpec with
    #[derive(Clone, Deserialize, Debug)]
    struct PodSpecSimple {
        containers: Vec<ContainerSimple>,
    }
    #[derive(Clone, Deserialize, Debug)]
    struct ContainerSimple {
        #[allow(dead_code)]
        image: String,
    }
    type PodSimple = Object<PodSpecSimple, NotUsed>;

    // Here we simply steal the type info from k8s_openapi, but we could create this from scratch.
    let ar = ApiResource::erase::<k8s_openapi::api::core::v1::Pod>(&());

    let pods: Api<PodSimple> = Api::namespaced_with(client, "default", &ar);
    for p in pods.list(&Default::default()).await? {
        info!("Found pod {} running: {:?}", p.name(), p.spec.containers);
    }

    Ok(())
}
