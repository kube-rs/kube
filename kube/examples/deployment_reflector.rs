#[macro_use] extern crate log;
use futures::TryStreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, ListParams, Meta},
    Client,
};
use kube_runtime::{reflector, reflector::ObjectRef, utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let store = reflector::store::Writer::<Deployment>::default();
    let reader = store.as_reader();
    let rf = reflector(
        store,
        watcher(
            Api::<Deployment>::namespaced(client.clone(), &namespace),
            ListParams::default().timeout(10), // short watch timeout in this example
        ),
    );

    // We can also interact with state in another thread
    let pref = ObjectRef::new_namespaced("prometheus-operator".to_string(), namespace.clone());
    // FIXME: ^ this is an awkward interface
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let prom = reader.get(&pref);
            info!("prometheus?: {:?}", prom);
            //let deploys: Vec<_> = reader.state().iter().map(|k| Meta::name(&k)).collect();
            //info!("Current deploys: {:?}", deploys);
            // FIXME: full state interface / weak ptr?
            // currently a full fetch feels limited by dashmap, don't want to expose that interface
            // also not sure what the benefit of it is atm..
        }
    });

    // We can look at the events we want and use it as a watcher
    let mut rfa = Box::pin(try_flatten_applied(rf));
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {:?}", event);
    }

    Ok(())
}
