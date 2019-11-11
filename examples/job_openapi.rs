#[macro_use] extern crate log;
use serde_json::json;

use kube::{
    api::{Api, PostParams, DeleteParams, ListParams, WatchEvent},
    client::{APIClient},
    config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    // Create a Job
    let my_job = json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {
            "name": "empty-job"
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

    let jobs = Api::v1Job(client).within("default");
    let pp = PostParams::default();

    let data = serde_json::to_vec(&my_job).expect("failed to serialize job");
    jobs.create(&pp, data).await.expect("failed to create job");

    // See if it ran to completion
    let lp = ListParams::default();
    jobs.watch(&lp, "").await.and_then(|res| {
        for status in res {
            match status {
                WatchEvent::Added(s) => {
                    info!("Added {}", s.metadata.name);
                },
                WatchEvent::Modified(s) => {
                    let current_status = s.status.clone().expect("Status is missing");
                    current_status.completion_time.and_then(|_| {
                        info!("Modified: {} is complete", s.metadata.name);
                        Some(())
                    }).or_else(|| {
                        info!("Modified: {} is running", s.metadata.name);
                        Some(())
                    });
                },
                WatchEvent::Deleted(s) => {
                    info!("Deleted {}", s.metadata.name);
                }
                WatchEvent::Error(s) => {
                    error!("{}", s);
                }
            }
        }
        Ok(())
    }).expect("Failed to watch");

    // Clean up the old job record..
    info!("Deleting the job record.");
    let dp = DeleteParams::default();
    jobs.delete("empty-job", &dp).await.expect("failed to delete job");
    Ok(())
}
