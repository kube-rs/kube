use k8s_openapi::api::batch::v1::Job;
use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions},
    Client,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let jobs: Api<Job> = Api::default_namespaced(client);

    info!("Creating job");
    let name = "empty-job";
    let data = serde_json::from_value(serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": name,
        },
        "spec": {
            "template": {
                "metadata": {
                    "name": "empty-job-pod"
                },
                "spec": {
                    "containers": [{
                        "name": "empty",
                        "image": "alpine:latest"
                    }],
                    "restartPolicy": "Never",
                }
            }
        }
    }))?;
    jobs.create(&PostParams::default(), &data).await?;

    info!("Waiting for job to complete");
    let cond = await_condition(jobs.clone(), name, conditions::is_job_completed());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(20), cond).await?;

    info!("Cleaning up job record");
    jobs.delete(name, &DeleteParams::background()).await?;
    Ok(())
}
