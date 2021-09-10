// Demonstrates deleting a resource and waiting for it to be finalized

use k8s_openapi::api::core::v1::Secret;
use kube::api::DeleteParams;
use kube_runtime::finalizer::finalize_and_delete;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    env_logger::init();
    let kube = kube::Client::try_default().await?;
    let cms = kube::Api::<Secret>::default_namespaced(kube.clone());
    finalize_and_delete(&cms, "kubers-deleter-example-secret", &DeleteParams::default()).await?;
    Ok(())
}
