#[macro_use] extern crate log;
use futures::StreamExt;
use k8s_openapi::{
    api::{
        batch::v1::JobSpec,
        core::v1::{Container, PodSpec, PodTemplateSpec},
    },
    apimachinery::pkg::apis::meta::v1::ObjectMeta as OpenApiObjectMeta,
};

use kube::{
    api::{Api, DeleteParams, ListParams, Object, ObjectMeta, PostParams, TypeMeta, WatchEvent},
    client::APIClient,
    config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    // Create a Job
    let my_job = Object {
        types: TypeMeta {
            apiVersion: Some("batch/v1".to_string()),
            kind: Some("Job".to_string()),
        },
        metadata: ObjectMeta {
            name: "empty-job".to_string(),
            ..Default::default()
        },
        spec: JobSpec {
            template: PodTemplateSpec {
                metadata: Some(OpenApiObjectMeta {
                    name: Some("empty-job-pod".to_string()),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "empty".to_string(),
                        image: Some("alpine:latest".to_string()),
                        ..Default::default()
                    }],
                    restart_policy: Some("Never".to_string()),
                    ..Default::default()
                }),
            },
            ..Default::default()
        },
        status: None,
    };

    let jobs = Api::v1Job(client).within("default");
    let pp = PostParams::default();

    jobs.create(&pp, &my_job).await.expect("failed to create job");

    // See if it ran to completion
    let lp = ListParams::default();
    let mut stream = jobs.watch(&lp, "").await?.boxed();

    while let Some(status) = stream.next().await {
        match status {
            WatchEvent::Added(s) => info!("Added {}", s.metadata.name),
            WatchEvent::Modified(s) => {
                let current_status = s.status.clone().expect("Status is missing");
                match current_status.completion_time {
                    Some(_) => info!("Modified: {} is complete", s.metadata.name),
                    _ => info!("Modified: {} is running", s.metadata.name),
                }
            }
            WatchEvent::Deleted(s) => info!("Deleted {}", s.metadata.name),
            WatchEvent::Error(s) => error!("{}", s),
        }
    }

    // Clean up the old job record..
    info!("Deleting the job record.");
    let dp = DeleteParams::default();
    jobs.delete("empty-job", &dp).await.expect("failed to delete job");
    Ok(())
}
