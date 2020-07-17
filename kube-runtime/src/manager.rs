use crate::{
    controller::{controller, trigger_owners, trigger_self, trigger_with, Context, Error, ReconcilerAction},
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef,
    },
    utils::{try_flatten_applied, try_flatten_touched},
    watcher::{self, watcher},
};
use futures::{stream, Stream, TryFuture, TryStream};
use kube::api::{Api, ListParams, Meta};
use serde::de::DeserializeOwned;
use std::pin::Pin;

/// A builder for controller
pub struct ControllerBuilder<K>
where
    K: Clone + Meta + 'static,
{
    // NB: Need to Unpin for stream::select_all
    // TODO: get an arbitrary std::error::Error in here?
    selector: Vec<Pin<Box<dyn Stream<Item = Result<ObjectRef<K>, watcher::Error>>>>>,
    reader: Store<K>,
}

impl<K> ControllerBuilder<K>
where
    K: Clone + Meta + DeserializeOwned + 'static,
{
    /// Create a ControllerBuilder on a type `K`
    ///
    /// Configure `ListParams` and `Api` so you only get reconcile events
    /// for the correct Api scope (cluster/all/namespaced), or ListParams subset
    ///
    /// A writer is exposed for convenience so you can peak into the main reflector's state.
    pub fn new(owned_api: Api<K>, writer: Writer<K>, lp: ListParams) -> Self {
        let reader = writer.as_reader();
        let mut selector = vec![];
        let self_watcher: Pin<Box<dyn Stream<Item = Result<ObjectRef<K>, watcher::Error>>>> = Box::pin(
            trigger_self(try_flatten_applied(reflector(writer, watcher(owned_api, lp)))),
        );
        selector.push(self_watcher);
        Self { selector, reader }
    }

    /// Indicate child objets K owns and be notified when they change
    ///
    /// This type `CHILD` must have OwnerReferences set to point back to `K`.
    /// You can customize the parameters used by the underlying watcher if
    /// only a subset of `CHILD` entries are required.
    /// The `api` must have the correct scope (cluster/all namespaces, or namespaced)
    pub fn owns<CHILD: Clone + Meta + DeserializeOwned + 'static>(
        mut self,
        api: Api<CHILD>,
        lp: ListParams,
    ) -> Self {
        let child_watcher = trigger_owners(try_flatten_touched(watcher(api, lp)));
        self.selector.push(Box::pin(child_watcher));
        self
    }

    /// Indicate an object to watch with a custom mapper
    ///
    /// This mapper should return something like Option<ObjectRef<K>>
    pub fn watches<
        OTHER: Clone + Meta + DeserializeOwned + 'static,
        I: 'static + IntoIterator<Item = ObjectRef<K>>,
    >(
        mut self,
        api: Api<OTHER>,
        lp: ListParams,
        mapper: impl Fn(OTHER) -> I + 'static,
    ) -> Self {
        let other_watcher = trigger_with(try_flatten_touched(watcher(api, lp)), mapper);
        self.selector.push(Box::pin(other_watcher));
        self
    }

    /// Consume the ControllerBuilder and start the controller stream
    ///
    /// This creates a stream from all builder calls and starts a controller with
    /// a specified `reconciler` and `error_policy` callbacks. Each of these will be called
    /// with your configurable `Context`.
    pub fn run<ReconcilerFut, T, QueueStream>(
        self,
        reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
        error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> ReconcilerAction,
        context: Context<T>,
    ) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error, watcher::Error>>>
    where
        K: Clone + Meta + 'static,
        ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
        ReconcilerFut::Error: std::error::Error + 'static,
        QueueStream: TryStream<Ok = ObjectRef<K>>,
        QueueStream::Error: std::error::Error + 'static,
    {
        let input_stream = stream::select_all(self.selector);
        controller(reconciler, error_policy, context, self.reader, input_stream)
    }
}
