#[macro_use] extern crate log;
use futures::StreamExt;
use k8s_openapi::api::batch::v1::Job;
use serde_json::json;

use kube::{
    api::{Api, DeleteParams, ListParams, Meta, PostParams, WatchEvent},
    client::APIClient,
    config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // Create a Job
    let job_name = "empty-job";
    let my_job = json!({
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
    });

    let jobs: Api<Job> = Api::namespaced(client, &namespace);
    let pp = PostParams::default();

    let data = serde_json::to_vec(&my_job).expect("failed to serialize job");
    jobs.create(&pp, data).await.expect("failed to create job");

    // See if it ran to completion
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", job_name)) // only want events for our job
        .timeout(20); // should be done by then
    let mut stream = jobs.watch(&lp, "").await?.boxed();

    while let Some(status) = stream.next().await {
        match status {
            WatchEvent::Added(s) => info!("Added {}", Meta::name(&s)),
            WatchEvent::Modified(s) => {
                let current_status = s.status.clone().expect("Status is missing");
                match current_status.completion_time {
                    Some(_) => info!("Modified: {} is complete", Meta::name(&s)),
                    _ => info!("Modified: {} is running", Meta::name(&s)),
                }
            }
            WatchEvent::Deleted(s) => info!("Deleted {}", Meta::name(&s)),
            WatchEvent::Error(s) => error!("{}", s),
        }
    }

    // Clean up the old job record..
    info!("Deleting the job record.");
    let dp = DeleteParams::default();
    jobs.delete("empty-job", &dp).await.expect("failed to delete job");
    Ok(())
}
