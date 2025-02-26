use futures::{future, stream, StreamExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Secret},
};
use kube::{
    api::ApiResource,
    runtime::{
        controller::Action, reflector::multi_dispatcher::MultiDispatcher, watcher, Controller,
        WatchStreamExt as _,
    },
    Api, Client, ResourceExt,
};
use std::{fmt::Debug, sync::Arc, time::Duration};
use thiserror::Error;
use tracing::*;

#[derive(Debug, Error)]
enum Infallible {}

// A generic reconciler that can be used with any object whose type is known at
// compile time. Will simply log its kind on reconciliation.
async fn reconcile<K>(_obj: Arc<K>, _ctx: Arc<()>) -> Result<Action, Infallible>
where
    K: ResourceExt<DynamicType = ()>,
{
    let kind = K::kind(&());
    info!("Reconciled {kind}");
    Ok(Action::await_change())
}

fn error_policy<K: ResourceExt>(_: Arc<K>, _: &Infallible, _ctx: Arc<()>) -> Action {
    info!("error");
    Action::requeue(Duration::from_secs(10))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let writer = MultiDispatcher::new(128);

    // multireflector stream
    let mut combo_stream = stream::select_all(vec![]);
    combo_stream.push(
        watcher::watcher(
            Api::all_with(client.clone(), &ApiResource::erase::<Deployment>(&())),
            Default::default(),
        )
        .boxed(),
    );

    // watching config maps, but ignoring in the final configuration
    combo_stream.push(
        watcher::watcher(
            Api::all_with(client.clone(), &ApiResource::erase::<ConfigMap>(&())),
            Default::default(),
        )
        .boxed(),
    );

    // Combine duplicate type streams with narrowed down selection
    combo_stream.push(
        watcher::watcher(
            Api::default_namespaced_with(client.clone(), &ApiResource::erase::<Secret>(&())),
            Default::default(),
        )
        .boxed(),
    );
    combo_stream.push(
        watcher::watcher(
            Api::namespaced_with(client.clone(), "kube-system", &ApiResource::erase::<Secret>(&())),
            Default::default(),
        )
        .boxed(),
    );

    let watcher = combo_stream.broadcast_shared(writer.clone());

    let (sub, reader) = writer.subscribe::<Deployment>();
    let deploy = Controller::for_shared_stream(sub, reader)
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled deployment {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile metadata"),
            };
        });

    let (sub, reader) = writer.subscribe::<Secret>();
    let secret = Controller::for_shared_stream(sub, reader)
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled secret {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile metadata"),
            };
        });

    info!("long watches starting");
    tokio::select! {
        r = watcher.for_each(|_| future::ready(())) => println!("watcher exit: {r:?}"),
        x = deploy => println!("deployments exit: {x:?}"),
        x = secret => println!("secrets exit: {x:?}"),
    }

    Ok(())
}
