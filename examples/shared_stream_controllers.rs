use std::{ops::Deref, sync::Arc, time::Duration};

use futures::{future, StreamExt};
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Pod};
use kube::{
    runtime::{
        controller::Action,
        predicates,
        reflector::{self, ReflectHandle},
        watcher, Config, Controller, WatchStreamExt,
    },
    Api, Client, ResourceExt,
};
use tracing::{debug, error, info, warn};

use thiserror::Error;

// Helper module that namespaces two constants describing a Kubernetes status condition
pub mod condition {
    pub static UNDOCUMENTED_TYPE: &str = "UndocumentedPort";
    pub static STATUS_TRUE: &str = "True";
}

const SUBSCRIBE_BUFFER_SIZE: usize = 256;

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

fn error_policy<K: ResourceExt>(obj: Arc<K>, error: &Infallible, _ctx: Arc<()>) -> Action {
    error!(%error, name = %obj.name_any(), "Failed reconciliation");
    Action::requeue(Duration::from_secs(10))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods = Api::<Pod>::all(client.clone());
    let config = Config::default().concurrency(2);

    // Create a shared store with a predefined buffer that will be shared between subscribers.
    let (reader, writer) = reflector::store_shared(SUBSCRIBE_BUFFER_SIZE);
    // Before threading an object watch through the store, create a subscriber.
    // Any number of subscribers can be created from one writer.
    let subscriber: ReflectHandle<Pod> = writer
        .subscribe()
        .expect("subscribers can only be created from shared stores");

    // Subscriber events can be filtered in advance with predicates
    let filtered = subscriber
        .clone()
        .map(|r| Ok(r.deref().clone()))
        .predicate_filter(predicates::resource_version)
        .filter_map(|r| future::ready(r.ok().map(Arc::new)));

    // Reflect a stream of pod watch events into the store and apply a backoff. For subscribers to
    // be able to consume updates, the reflector must be shared.
    let pod_watch = watcher(pods.clone(), Default::default())
        .default_backoff()
        .reflect_shared(writer)
        .for_each(|res| async move {
            match res {
                Ok(event) => debug!("Received event on root stream {event:?}"),
                Err(error) => error!(%error, "Unexpected error when watching resource"),
            }
        });

    // Create the first controller; the controller will log whenever it
    // reconciles a pod. The reconcile is a no-op.
    // Controllers accept subscribers through a dedicated interface.
    let pod_controller = Controller::for_shared_stream(filtered, reader)
        .with_config(config.clone())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|res| async move {
            match res {
                Ok(v) => debug!("Reconciled pod {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile metadata"),
            }
        });

    // Create the second controller; the controller will log whenever it
    // reconciles a deployment. Any changes to a pod will trigger a
    // reconciliation to the owner (a deployment). Reconciliations are no-op.
    let deploys = Api::<Deployment>::all(client.clone());
    let deploy_controller = Controller::new(deploys, Default::default())
        .with_config(config)
        .owns_shared_stream(subscriber)
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|res| async move {
            match res {
                Ok(v) => debug!("Reconciled deployment {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile status"),
            }
        });

    // Drive streams to readiness. The initial watch (that is reflected) needs to be driven to
    // consume events from the API Server and forward them to subscribers.
    //
    // Both controllers will operate on shared objects.
    tokio::select! {
        _ = futures::future::join(pod_controller, deploy_controller) => {},
        _ = pod_watch => {}
    }

    Ok(())
}
