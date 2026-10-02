use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client, ResourceExt,
    api::{Api, DeleteParams, Patch, PatchParams, PostParams},
    runtime::wait::{await_condition, conditions},
};
use tracing::*;

fn inspect_pod_resize(pod: &Pod) {
    if let Some(spec) = &pod.spec {
        if let Some(container) = spec.containers.first() {
            info!("Spec resources (desired): {:?}", container.resources);
        }
    }
    if let Some(status) = &pod.status {
        info!("Resize status: {:?}", status.resize);
        if let Some(container_status) = status.container_statuses.as_ref().and_then(|cs| cs.first()) {
            info!("Status resources (actual): {:?}", container_status.resources);
            info!("Container restart count: {}", container_status.restart_count);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods: Api<Pod> = Api::default_namespaced(client);

    // Create a sample pod with resource limits and resize policy
    info!("Creating pod with initial resource requirements");
    let pod_template = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "resize-wait-demo" },
        "spec": {
            "containers": [{
                "name": "app",
                "image": "alpine:3.23",
                "resizePolicy": [
                    {
                        "resourceName": "cpu",
                        "restartPolicy": "NotRequired"
                    },
                    {
                        "resourceName": "memory",
                        "restartPolicy": "RestartContainer"
                    }
                ],
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
    });
    let pod = serde_json::from_value(pod_template)?;
    let pp = PostParams::default();
    match pods.create(&pp, &pod).await {
        Ok(created) => info!("Created pod: {}", created.name_any()),
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("Pod already exists, patching it...");
            let pp = PatchParams::apply("pod-resize-example");
            let patch = Patch::Apply(pod);
            let _ = pods.patch("resize-wait-demo", &pp, &patch).await?;
        }
        Err(e) => return Err(e.into()),
    }
    // Wait for pod to be running
    info!("Waiting for pod to be running...");
    let running = await_condition(pods.clone(), "resize-wait-demo", conditions::is_pod_running());
    tokio::time::timeout(std::time::Duration::from_secs(60), running).await??;
    info!("✓ Pod is running");

    // Display initial resources
    let current = pods.get("resize-wait-demo").await?;
    info!("Initial pod state:");
    inspect_pod_resize(&current);

    // Resize CPU (no restart required)
    info!("\n--- Example 1: Resizing CPU (NotRequired restart policy) ---");
    let cpu_patch = serde_json::json!({
        "spec": {
            "containers": [{
                "name": "app",
                "resources": {
                    "requests": {
                        "cpu": "150m"
                    },
                    "limits": {
                        "cpu": "300m"
                    }
                }
            }]
        }
    });

    let patch_params = PatchParams::default();
    info!("Patching pod with new CPU resources...");
    pods.patch_resize("resize-wait-demo", &patch_params, &Patch::Strategic(cpu_patch))
        .await?;

    info!("Waiting for resize to complete...");
    let resized = await_condition(pods.clone(), "resize-wait-demo", conditions::is_pod_resized());
    let pod = tokio::time::timeout(std::time::Duration::from_secs(30), resized).await??;
    info!("✓ Pod CPU resize completed successfully!");
    if let Some(pod) = pod {
        inspect_pod_resize(&pod);
    }

    // Resize Memory (restart required)
    info!("\n--- Example 2: Resizing Memory (RestartContainer policy) ---");
    let mem_patch = serde_json::json!({
        "spec": {
            "containers": [{
                "name": "app",
                "resources": {
                    "requests": {
                        "memory": "192Mi"
                    },
                    "limits": {
                        "memory": "384Mi"
                    }
                }
            }]
        }
    });

    info!("Patching pod with new memory resources...");
    pods.patch_resize("resize-wait-demo", &patch_params, &Patch::Strategic(mem_patch))
        .await?;

    info!("Waiting for memory resize to complete (container will restart)...");
    let resized = await_condition(pods.clone(), "resize-wait-demo", conditions::is_pod_resized());
    let pod = tokio::time::timeout(std::time::Duration::from_secs(60), resized).await??;
    info!("✓ Pod memory resize completed successfully!");
    if let Some(pod) = pod {
        inspect_pod_resize(&pod);
    }

    // Cleanup
    info!("\nCleaning up...");
    let dp = DeleteParams::default();
    pods.delete("resize-wait-demo", &dp).await?;
    info!("Pod deleted");

    Ok(())
}
