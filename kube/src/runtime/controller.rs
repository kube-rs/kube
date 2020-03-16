use crate::{
    api::{
        resource::{ListParams, Resource},
        Meta, WatchEvent,
    },
    client::APIClient,
    runtime::informer::Informer,
    Error, Result,
};
use futures::{lock::Mutex, stream, Stream, StreamExt};
use futures_timer::Delay;
use serde::de::DeserializeOwned;
use std::{convert::TryFrom, time::Duration};
use tokio::sync::mpsc;

/// An object to be reconciled
///
/// The type that is pulled out of Controller::poll
#[derive(Debug, Clone)]
pub struct ReconcileEvent {
    pub name: String,
    pub namespace: Option<String>,
}

impl<K> From<K> for ReconcileEvent
where
    K: Meta,
{
    fn from(k: K) -> ReconcileEvent {
        ReconcileEvent {
            name: Meta::name(&k),
            namespace: Meta::namespace(&k),
        }
    }
}

impl<K> TryFrom<WatchEvent<K>> for ReconcileEvent
where
    K: Meta + Clone,
{
    type Error = crate::Error;

    /// Helper to convert the openapi ReplicaSet to the useful info
    fn try_from(w: WatchEvent<K>) -> Result<ReconcileEvent> {
        match w {
            WatchEvent::Added(o) => Ok(o.into()),
            WatchEvent::Modified(o) => Ok(o.into()),
            WatchEvent::Deleted(o) => Ok(o.into()),
            WatchEvent::Error(e) => Err(Error::Api(e)),
        }
    }
}

/// An Ok return value from a reconcile fn
///
/// Designed so the Controller can decide  whether to requeue the event
/// Error cases are not encapsulated in this struct (they are handled by Result)
#[derive(Debug)]
pub enum ReconcileStatus {
    /// Successful reconcile
    Complete,
    ///  Partial success, reque after some time
    RequeAfter(Duration),
}

/// A controller for a kubernetes object K
#[derive(Clone)]
pub struct Controller<K, F>
where
    K: Clone + DeserializeOwned + Meta,
    F: Fn(ReconcileEvent) -> Result<ReconcileStatus>,
{
    client: APIClient,
    resource: Resource,
    informers: Vec<Informer<K>>,
    reconciler: Box<F>,
}

// TODO: is 'static limiting here?
impl<K: 'static, F: 'static> Controller<K, F>
where
    K: Clone + DeserializeOwned + Meta + Send + Sync,
    F: Fn(ReconcileEvent) -> Result<ReconcileStatus> + Send + Sync,
{
    /// Create a controller with a kube client on a kube resource
    pub fn new(client: APIClient, r: Resource, recfn: F) -> Self {
        Controller {
            client: client,
            resource: r,
            informers: vec![],
            reconciler: Box::new(recfn),
        }
    }

    /// Create internal informers for an associated kube resource
    ///
    /// TODO: this needs to only find resources with a property matching root resource
    pub fn owns(mut self, r: Resource, lp: ListParams) -> Self {
        self.informers.push(Informer::new(self.client.clone(), lp, r));
        self
    }

    /// Initialize
    pub fn init(self) {
        info!("Starting Controller for {:?}", self.resource);
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 1. poll informers in parallel and push results to queue
        for inf in self.informers.clone() {
            // TODO: ownership move?
            //let queue = self.queue.clone();
            let txi = tx.clone();
            tokio::spawn(async move {
                let mut poll_i = inf.poll().await.expect("could talk to api server").boxed();
                while let Some(ev) = poll_i.next().await {
                    match ev {
                        Ok(wi) => {
                            let ri = ReconcileEvent::try_from(wi);
                            //(*queue.lock().await).push_back(ri);
                            txi.send(ri).expect("channel can receive");
                        }
                        _ => unimplemented!(),
                        //Err(e) => tx.unbounded_send(Err(e)),
                    }
                }
            });
        }
        // TODO: init main informer
        // TODO: queue up events
        // TODO: debounce events


        // Prepare a sender so we can stack things back on the channel
        let txa = tx.clone();
        // Event loop that triggers the reconcile fn
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    None => return, // tx dropped
                    Some(wi) => {
                        match wi {
                            Err(e) => {
                                // Got WatchEvent::Error
                                // TODO: handle here? handle above?
                            }
                            Ok(wo) => {
                                let ri = wo.clone();
                                let name = wo.name.clone();
                                match (self.reconciler)(wo) {
                                    Ok(status) => {
                                        // Reconcile cb completed with app decicion
                                        match status {
                                            ReconcileStatus::Complete => {
                                                info!("Reconciled {}", name);
                                            }
                                            ReconcileStatus::RequeAfter(dur) => {
                                                info!("Requeing {} in {:?}", name, dur);
                                                Delay::new(dur).await;
                                                txa.send(Ok(ri)).expect("channel can receive");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // Reconcile cb failed (any unspecified error)
                                        let dur = Duration::from_secs(10); // TODO: expo decay
                                        warn!("Failed to reconcile {} - requining in {:?}", e, dur);
                                        Delay::new(dur).await;
                                        txa.send(Ok(ri)).expect("channel can receive");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
