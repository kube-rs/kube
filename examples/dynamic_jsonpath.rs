use kube::{
    api::{Api, DynamicObject, GroupVersionKind, ListParams, ResourceExt},
    core::{ApiResource},
    discovery,
    Client,
};
use jsonpath_lib;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    // Here we simply steal the type info from k8s_openapi, but we could create this from scratch.
    let ar = ApiResource::erase::<k8s_openapi::api::core::v1::Pod>(&());

    let field_selector = std::env::var("FIELD_SELECTOR").unwrap_or_default();
    let jsonpath = format!("{}{}", "$", std::env::var("JSONPATH").unwrap_or_else(|_|"@".into()));

    // Use the discovered kind in an Api with the ApiResource as its DynamicType
    let api = Api::<DynamicObject>::all_with(client, &ar);
    let list_params = ListParams::default().fields(&*field_selector);
    let list = api.list(&list_params).await?;

    // Use the given JSONPATH to filter the ObjectList
    let items = serde_json::to_value(&list.items)?;
    let res = jsonpath_lib::select(&items, &*jsonpath).unwrap();
    println!("\t\t {:?}", res);
    Ok(())
}
