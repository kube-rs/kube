use clap::StructOpt;
use k8s_openapi::api::coordination::v1::Lease;
use kube::runtime::Elector;
use tokio::signal::ctrl_c;

#[derive(clap::Parser)]
struct Opts {
    #[clap(long)]
    lease: String,
    #[clap(long)]
    namespace: Option<String>,
    #[clap(long)]
    instance: String,
    #[clap(long)]
    lease_duration_secs: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    tracing_subscriber::fmt::init();

    let kube = kube::Client::try_default().await?;

    let Opts {
        lease,
        namespace,
        instance,
        lease_duration_secs,
    } = Opts::parse();

    tracing::info!(?namespace, ?lease, ?instance, "acquiring lease...");
    let leases = if let Some(ns) = namespace {
        kube::Api::<Lease>::namespaced(kube, &ns)
    } else {
        kube::Api::<Lease>::default_namespaced(kube)
    };
    let elector = Elector::new(leases, &lease, &instance, lease_duration_secs);
    elector
        .run(async {
            tracing::info!("acquired lease, press <ctrl+c> to release...");
            ctrl_c().await
        })
        .await??;
    tracing::info!("released lease!");

    Ok(())
}
