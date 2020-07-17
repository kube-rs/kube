use crate::{
    controller::{controller, trigger_owners, trigger_self, Context, Error, ReconcilerAction},
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
    //E: std::error::Error + 'static,
    //QueueStream: TryStream<Ok = ObjectRef<K>>,
    //QueueStream::Error: std::error::Error + 'static,
{
    // NB: Need to Unpin for stream::select_all
    selector: Vec<Pin<Box<dyn Stream<Item = Result<ObjectRef<K>, watcher::Error>>>>>,
    reader: Store<K>,
}

impl<K> ControllerBuilder<K>
where
    K: Clone + Meta + DeserializeOwned + 'static,
    //E: std::error::Error + 'static,
    //QueueStream: TryStream<Ok = ObjectRef<K>>,
    //QueueStream::Error: std::error::Error + 'static
{
    fn new(owned_api: Api<K>, writer: Writer<K>, lp: ListParams) -> Self {
        let reader = writer.as_reader();
        let mut selector = vec![];
        let self_watcher: Pin<Box<dyn Stream<Item = Result<ObjectRef<K>, watcher::Error>>>> = Box::pin(
            trigger_self(try_flatten_applied(reflector(writer, watcher(owned_api, lp)))),
        );
        selector.push(self_watcher);
        Self { selector, reader }
    }

    fn owns<KOwns: Clone + Meta + DeserializeOwned + 'static>(
        mut self,
        api: Api<KOwns>,
        lp: ListParams,
    ) -> Self {
        let child_watcher = trigger_owners(try_flatten_touched(watcher(api, lp)));
        self.selector.push(Box::pin(child_watcher));
        self
    }

    // TODO: fn watches and arbitrary stream?

    /// Consume the ControllerBuilder and create the controller
    fn run<ReconcilerFut, T, QueueStream>(
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
        // TODO: how to get Items in this stream to have the same opaque type?
        let input_stream = stream::select_all(self.selector);
        controller(reconciler, error_policy, context, self.reader, input_stream)
    }
}
