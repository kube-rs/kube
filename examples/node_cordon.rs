#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
    runtime::{reflector, utils::try_flatten_applied, watcher},
    Client,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let node_name = "node1";
    let first_node = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Node",
        "metadata": {
            "name": node_name,
        },
    }))?;

    let nodes: Api<Node> = Api::all(client.clone());
    nodes.create(&PostParams::default(), &first_node).await?;

    let list_params = ListParams::default().fields(&format!("spec.unschedulable==false"));
    let nodes_init = nodes.list(&list_params).await?;
    let num_nodes_before_cordon = nodes_init.items.len();

    nodes.cordon(node_name).await?;
    let nodes_after_cordon = nodes.list(&list_params).await?;
    assert_eq!(nodes_after_cordon.items.len(), num_nodes_before_cordon - 1);

    nodes.uncordon(node_name).await?;
    let nodes_after_uncordon = nodes.list(&list_params).await?;
    assert_eq!(nodes_after_uncordon.items.len(), num_nodes_before_cordon);
    nodes.delete(node_name, &DeleteParams::default());
    Ok(())
}
