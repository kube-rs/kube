use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::{
    api::{Api, LogParams},
    Client,
};
use tracing::*;

/// limited variant of kubectl logs
#[derive(clap::Parser)]
struct App {
    #[arg(long, short = 'c')]
    container: Option<String>,

    #[arg(long, short = 't')]
    tail: Option<i64>,

    #[arg(long, short = 'f')]
    follow: bool,

    /// Since seconds
    #[arg(long, conflicts_with = "since_time")]
    since: Option<i64>,
    /// Since time
    #[arg(long, conflicts_with = "since")]
    since_time: Option<DateTime<Utc>>,

    /// Include timestamps in the log output
    #[arg(long, default_value = "false")]
    timestamps: bool,

    pod: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let app: App = clap::Parser::parse();
    let client = Client::try_default().await?;

    info!("Fetching logs for {:?}", app.pod);
    let pods: Api<Pod> = Api::default_namespaced(client);
    let mut logs = pods
        .log_stream(&app.pod, &LogParams {
            follow: app.follow,
            container: app.container,
            tail_lines: app.tail,
            since_seconds: app.since,
            since_time: app.since_time,
            timestamps: app.timestamps,
            ..LogParams::default()
        })
        .await?
        .lines();

    while let Some(line) = logs.try_next().await? {
        println!("{}", line);
    }
    Ok(())
}
