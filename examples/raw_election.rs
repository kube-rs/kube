use std::time::Duration;

use k8s_openapi::{api::coordination::v1::Lease, chrono::Utc};
use kube::runtime::lock::raw::{LockSettings, RawLock};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let kube = kube::Client::try_default().await?;

    let lease = std::env::var("LEASE").unwrap_or_else(|_| "kube-election-example".to_string());
    let namespace = std::env::var("LEASE_NS").ok();
    let instance = std::env::var("INSTANCE").unwrap_or_else(|_| std::process::id().to_string());
    let lease_duration_secs = std::env::var("LEASE_DURATION")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap();

    let leases = if let Some(ns) = &namespace {
        kube::Api::<Lease>::namespaced(kube, ns)
    } else {
        kube::Api::<Lease>::default_namespaced(kube)
    };

    let settings = LockSettings {
        lease_name: lease.clone(),
        identity: instance.clone(),
        expiration_timeout_secs: lease_duration_secs,
    };

    let mut lock = RawLock::new(leases, settings);
    tracing::info!(?namespace, ?lease, ?instance, "Starting loop");
    let retirement_threshold = 10;
    let mut budget = retirement_threshold;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let now = Utc::now();
        let res = lock.try_acquire(now).await?;
        if res {
            tracing::info!(?instance, term = lock.term(), "I am the leader");
            budget -= 1;
            if budget == 0 {
                tracing::info!(?instance, "retiring");
                budget = retirement_threshold;
                let retire_res = lock.try_release().await?;
                if retire_res {
                    tracing::info!(?instance, "retired");
                    tokio::time::sleep(Duration::from_secs(8)).await;
                } else {
                    tracing::info!(?instance, "failed to retire");
                }
            }
        } else {
            tracing::info!(
                ?instance,
                leader = lock.owner(),
                term = lock.term(),
                "I am the follower"
            );
        }
    }
}
