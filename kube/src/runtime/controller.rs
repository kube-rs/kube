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
use serde::de::DeserializeOwned;
use std::convert::TryFrom;
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

/// A controller for a kubernetes object K
pub struct Controller<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    client: APIClient,
    resource: Resource,
    informers: Vec<Informer<K>>,
    channel: (
        mpsc::UnboundedSender<Result<ReconcileEvent>>,
        mpsc::UnboundedReceiver<Result<ReconcileEvent>>,
    ),
}

// TODO: is 'static limiting here?
impl<K: 'static> Controller<K>
where
    K: Clone + DeserializeOwned + Meta + Send + Sync,
{
    /// Create a controller with a kube client on a kube resource
    pub fn new(client: APIClient, r: Resource) -> Self {
        Controller {
            client: client,
            resource: r,
            informers: vec![],
            channel: mpsc::unbounded_channel(),
        }
    }

    /// Create internal informers for an associated kube resource
    ///
    /// TODO: this needs to only find resources with a property matching root resource
    pub fn owns(mut self, r: Resource, lp: ListParams) -> Self {
        self.informers.push(Informer::new(self.client.clone(), lp, r));
        self
    }

    /// Poll reconcile events through all internal informers
    pub async fn poll(&self) -> Result<impl Stream<Item = ReconcileEvent>> {
        // TODO: debounce rx events
        let rx = &self.channel.1;
        let stream = futures::stream::unfold(rx, |mut rx| async move {
            //async {
            match rx.recv().await {
                Some(w) => return Some((w, rx)),
                None => return None,
            }
            //}
        });
        Ok(stream.map(futures::stream::iter).flatten())
    }

    /// Initialize
    pub fn init(self) -> Self {
        info!("Starting Controller for {:?}", self.resource);

        // 1. poll informers in parallel and push results to queue
        for inf in self.informers.clone() {
            // TODO: ownership move?
            //let queue = self.queue.clone();
            let tx = self.channel.0.clone();
            tokio::spawn(async move {
                let mut poll_i = inf.poll().await.unwrap().boxed();
                while let Some(ev) = poll_i.next().await {
                    match ev {
                        Ok(wi) => {
                            let ri = ReconcileEvent::try_from(wi);
                            //(*queue.lock().await).push_back(ri);
                            tx.send(ri).expect("channel can receive");
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
        // TODO: trigger events
        self
    }
}
