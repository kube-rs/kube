use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams, PostParams, ResourceExt},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods: Api<Pod> = Api::default_namespaced(client);

    // Create a sample pod with resource limits
    info!("Creating pod with initial resource requirements");
    let pod: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "resize-demo" },
        "spec": {
            "containers": [{
                "name": "app",
                "image": "nginx:1.14.2",
                "resources": {
                    "requests": {
                        "cpu": "100m",
                        "memory": "128Mi"
                    },
                    "limits": {
                        "cpu": "200m",
                        "memory": "256Mi"
                    }
                }
            }]
        }
    }))?;

    let pp = PostParams::default();
    pods.create(&pp, &pod).await?;

    // Wait for pod to be running
    info!("Waiting for pod to be running...");
    let running = await_condition(pods.clone(), "resize-demo", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), running).await?;

    // Example 1: Using get_resize to view current state
    info!("Example 1: Getting pod resize subresource");
    let current = pods.get_resize("resize-demo").await?;
    if let Some(spec) = &current.spec
        && let Some(container) = spec.containers.first()
    {
        info!("Current resources: {:?}", container.resources);
    }

    // Example 2: Using patch_resize to update resources
    info!("Example 2: Patching pod resources using resize subresource");
    let patch = serde_json::json!({
        "spec": {
            "containers": [{
                "name": "app",
                "resources": {
                    "requests": {
                        "cpu": "150m",
                        "memory": "256Mi"
                    },
                    "limits": {
                        "cpu": "300m",
                        "memory": "512Mi"
                    }
                }
            }]
        }
    });

    let patch_params = PatchParams::default();
    match pods
        .patch_resize("resize-demo", &patch_params, &Patch::Strategic(patch))
        .await
    {
        Ok(resized) => {
            info!("Successfully patched pod: {}", resized.name_any());
            if let Some(spec) = resized.spec
                && let Some(container) = spec.containers.first()
            {
                info!("Updated resources via patch: {:?}", container.resources);
            }
        }
        Err(e) => {
            error!("Failed to patch resize pod: {}", e);
        }
    }

    // Example 3: Using replace_resize
    info!("Example 3: Using replace_resize method");
    let mut current_pod = pods.get_resize("resize-demo").await?;

    if let Some(spec) = &mut current_pod.spec
        && let Some(container) = spec.containers.get_mut(0)
        && let Some(resources) = &mut container.resources
        && let Some(requests) = &mut resources.requests
    {
        // Update memory request
        requests.insert(
            "memory".to_string(),
            k8s_openapi::apimachinery::pkg::api::resource::Quantity("384Mi".to_string()),
        );
    }

    match pods.replace_resize("resize-demo", &pp, &current_pod).await {
        Ok(resized) => {
            info!("Pod resized via replace: {}", resized.name_any());
            if let Some(spec) = resized.spec
                && let Some(container) = spec.containers.first()
            {
                info!("Final resources via replace: {:?}", container.resources);
            }
        }
        Err(e) => error!("Failed to replace_resize: {}", e),
    }

    // Cleanup
    info!("Cleaning up");
    let dp = DeleteParams::default();
    pods.delete("resize-demo", &dp).await?;

    Ok(())
}
