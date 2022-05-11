use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::batch::v1::Job;
use serde_json::json;
use tracing::*;

use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // Create a Job
    let job_name = "empty-job";
    let my_job = serde_json::from_value(json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": job_name,
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

    let jobs: Api<Job> = Api::default_namespaced(client);
    let pp = PostParams::default();

    jobs.create(&pp, &my_job).await?;

    // See if it ran to completion
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", job_name)) // only want events for our job
        .timeout(20); // should be done by then
    let mut stream = jobs.watch(&lp, "").await?.boxed();

    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(s) => info!("Added {}", s.name()),
            WatchEvent::Modified(s) => {
                let current_status = s.status.clone().expect("Status is missing");
                match current_status.completion_time {
                    Some(_) => {
                        info!("Modified: {} is complete", s.name());
                        break;
                    }
                    _ => info!("Modified: {} is running", s.name()),
                }
            }
            WatchEvent::Deleted(s) => info!("Deleted {}", s.name()),
            WatchEvent::Error(s) => error!("{}", s),
            _ => {}
        }
    }

    // Clean up the old job record..
    info!("Deleting the job record.");
    jobs.delete("empty-job", &DeleteParams::background().dry_run())
        .await?;
    jobs.delete("empty-job", &DeleteParams::background()).await?;
    Ok(())
}
