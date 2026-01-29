use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams, PostParams, ResourceExt},
    runtime::wait::{await_condition, conditions},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods: Api<Pod> = Api::default_namespaced(client);

    // Create a sample pod with resource limits and resize policy
    info!("Creating pod with initial resource requirements");
    let pod: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "resize-wait-demo" },
        "spec": {
            "containers": [{
                "name": "app",
                "image": "nginx:1.14.2",
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
    }))?;

    let pp = PostParams::default();
    match pods.create(&pp, &pod).await {
        Ok(created) => info!("Created pod: {}", created.name_any()),
        Err(kube::Error::Api(ae)) if ae.code == 409 => {
            info!("Pod already exists, deleting and recreating...");
            pods.delete("resize-wait-demo", &DeleteParams::default()).await?;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            pods.create(&pp, &pod).await?;
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
    if let Some(status) = &current.status
        && let Some(container_status) = status.container_statuses.as_ref().and_then(|cs| cs.first())
    {
        info!("Initial container resources: {:?}", container_status.resources);
        info!("Initial resize status: {:?}", status.resize);
    }

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
    match tokio::time::timeout(std::time::Duration::from_secs(30), resized).await {
        Ok(Ok(Some(pod))) => {
            info!("✓ Pod resize completed successfully!");
            if let Some(status) = &pod.status {
                info!("Resize status: {:?}", status.resize);
                if let Some(container_status) = status.container_statuses.as_ref().and_then(|cs| cs.first()) {
                    info!("Container resources after CPU resize: {:?}", container_status.resources);
                    info!("Container restart count: {}", container_status.restart_count);
                }
            }
        }
        Ok(Ok(None)) => warn!("Pod was deleted during resize wait"),
        Ok(Err(e)) => error!("Failed waiting for resize: {}", e),
        Err(_) => {
            warn!("Timeout waiting for resize to complete");
            let pod = pods.get("resize-wait-demo").await?;
            if let Some(status) = &pod.status {
                warn!("Current resize status: {:?}", status.resize);
                if let Some(conditions) = &status.conditions {
                    for cond in conditions {
                        if cond.type_ == "PodResizePending" || cond.type_ == "PodResizeInProgress" {
                            warn!("Resize condition: type={}, status={}, reason={:?}, message={:?}",
                                cond.type_, cond.status, cond.reason, cond.message);
                        }
                    }
                }
            }
        }
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
    match tokio::time::timeout(std::time::Duration::from_secs(60), resized).await {
        Ok(Ok(Some(pod))) => {
            info!("✓ Pod memory resize completed successfully!");
            if let Some(status) = &pod.status {
                info!("Resize status: {:?}", status.resize);
                if let Some(container_status) = status.container_statuses.as_ref().and_then(|cs| cs.first()) {
                    info!("Container resources after memory resize: {:?}", container_status.resources);
                    info!("Container restart count: {} (should be >0)", container_status.restart_count);
                }
            }
        }
        Ok(Ok(None)) => warn!("Pod was deleted during resize wait"),
        Ok(Err(e)) => error!("Failed waiting for resize: {}", e),
        Err(_) => {
            warn!("Timeout waiting for resize to complete");
            let pod = pods.get("resize-wait-demo").await?;
            if let Some(status) = &pod.status {
                warn!("Current resize status: {:?}", status.resize);
            }
        }
    }

    // Cleanup
    info!("\nCleaning up...");
    let dp = DeleteParams::default();
    pods.delete("resize-wait-demo", &dp).await?;
    info!("Pod deleted");

    Ok(())
}
