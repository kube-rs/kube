use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, PostParams, ResourceExt},
    Client, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    unsafe { std::env::set_var("RUST_LOG", "info,kube=debug"); }
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let name = std::env::args()
        .nth(1)
        .expect("Usage: cargo run --bin pod_resize <pod-name>");

    let pods: Api<Pod> = Api::default_namespaced(client);

    // Resize is only available in Kubernetes 1.33+
    k8s_openapi::k8s_if_ge_1_33! {
        tracing::info!("Resizing pod {}", name);

        // Get the current pod
        let mut pod = pods.get(&name).await?;
        tracing::info!("Current pod: {}", pod.name_any());

        // Modify the pod's resource requirements
        if let Some(ref mut spec) = pod.spec {
            if let Some(container) = spec.containers.get_mut(0) {
                // Example: Update CPU and memory limits
                if container.resources.is_none() {
                    container.resources = Some(Default::default());
                }
                if let Some(ref mut resources) = container.resources {
                    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
                    use std::collections::BTreeMap;

                    // Set new resource limits
                    let mut limits = BTreeMap::new();
                    limits.insert("cpu".to_string(), Quantity("500m".to_string()));
                    limits.insert("memory".to_string(), Quantity("256Mi".to_string()));
                    resources.limits = Some(limits);

                    // Set new resource requests
                    let mut requests = BTreeMap::new();
                    requests.insert("cpu".to_string(), Quantity("250m".to_string()));
                    requests.insert("memory".to_string(), Quantity("128Mi".to_string()));
                    resources.requests = Some(requests);
                }
            }
        }

        // Apply the resize
        let pp = PostParams::default();
        let updated_pod = pods.resize(&name, &pp, &pod).await?;
        tracing::info!("Pod resized successfully: {}", updated_pod.name_any());

        if let Some(ref spec) = updated_pod.spec {
            if let Some(container) = spec.containers.get(0) {
                if let Some(ref resources) = container.resources {
                    tracing::info!("New limits: {:?}", resources.limits);
                    tracing::info!("New requests: {:?}", resources.requests);
                }
            }
        }
    }

    Ok(())
}
