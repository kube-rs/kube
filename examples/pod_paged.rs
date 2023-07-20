use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, ResourceExt},
    Client,
};
use tracing::*;

// This example shows how to do pagination with the raw `Api` only.
// In many realistic setups that need a continual, paginated, safe list-watch;
// the `watcher` is an easier abstraction that has configurable pagination built in.

const PAGE_SIZE: u32 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let api = Api::<Pod>::default_namespaced(client);

    let mut continue_token: Option<String> = None;
    for page in 1.. {
        info!("Fetching Page #{page}");
        continue_token = fetch_page(&api, continue_token).await?;

        if continue_token.is_none() {
            info!("End of list");
            break;
        }
    }

    Ok(())
}

async fn fetch_page(api: &Api<Pod>, continue_token: Option<String>) -> anyhow::Result<Option<String>> {
    let mut lp = ListParams::default().limit(PAGE_SIZE);
    if let Some(token) = continue_token {
        lp = lp.continue_token(&token);
    }

    let pods = api.list(&lp).await?;
    let continue_token = pods.metadata.continue_.clone();
    for p in pods {
        info!("Found Pod: {}", p.name_any());
    }

    Ok(continue_token)
}
