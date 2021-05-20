use kube::{
    api::{Api, ApiResource, NotUsed, Object, ResourceExt},
    Client,
};
use log::info;
use serde::Deserialize;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    // Here we replace heavy type k8s_openapi::api::core::v1::PodSpec with
    #[derive(Clone, Deserialize, Debug)]
    struct PodSpecSimple {
        containers: Vec<ContainerSimple>,
    }
    #[derive(Clone, Deserialize, Debug)]
    struct ContainerSimple {
        image: String,
    }
    type PodSimple = Object<PodSpecSimple, NotUsed>;

    let ar = ApiResource::erase::<k8s_openapi::api::core::v1::Pod>(&());
    let pods: Api<PodSimple> = Api::namespaced_with(client, "default", &ar);
    for p in pods.list(&Default::default()).await? {
        info!("Found pod {} running: {:?}", p.name(), p.spec.containers);
    }

    Ok(())
}
