use kube::{
    api::{Api, ApiResource, NotUsed, Object, ResourceExt},
    k8s::corev1::Pod,
};
use serde::Deserialize;
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;

    // A slimmed down k8s::corev1::PodSpec
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

    // Steal the type info, to avoid hand-typing / using discovery.
    let ar = ApiResource::erase::<Pod>(&());

    let pods: Api<PodSimple> = Api::default_namespaced_with(client, &ar);
    for p in pods.list(&Default::default()).await? {
        info!("Pod {} runs: {:?}", p.name_any(), p.spec.containers);
    }

    Ok(())
}
