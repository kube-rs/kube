use std::pin::Pin;
use serde::de::DeserializeOwned;
use kube::api::ListParams;
use crate::{
    reflector::{reflector, ObjectRef, store::{Store, Writer}},
    watcher::{watcher},
    controller::{Context, Error, ReconcilerAction, controller, trigger_owners, trigger_self},
    utils::{try_flatten_applied, try_flatten_touched},
};
use futures::{
    future, stream, FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt, TryStream,
    TryStreamExt,
};
use kube::api::{Api, Meta};
use snafu::{futures::TryStreamExt as SnafuTryStreamExt, Backtrace, OptionExt, ResultExt, Snafu};


/// A builder for controller
pub struct ControllerBuilder<K, QueueStream> where
    K: Clone + Meta + 'static,
    QueueStream: TryStream<Ok = ObjectRef<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    // NB: Need to Unpin for stream::select_all
    // TODO: how to actually box this up?...
    pub(crate) selector: Vec<Pin<Box<QueueStream>>>,
    pub(crate) reader: Store<K>
}

impl<K, QueueStream> ControllerBuilder<K, QueueStream> where
    K: Clone + Meta + DeserializeOwned + 'static,
    QueueStream: TryStream<Ok = ObjectRef<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    fn new(owned_api: Api<K>, writer: Writer<K>, lp: ListParams) -> Self {
        let reader = writer.as_reader();
        let mut selector = vec![];
        let self_watcher = trigger_self(try_flatten_applied(reflector(writer, watcher(owned_api, lp))));
        selector.push(Box::pin(self_watcher));
        Self { selector, reader }
    }
    fn owns<KOwns: Clone + Meta + DeserializeOwned>(mut self, api: Api<KOwns>, lp: ListParams) -> Self {
        let child_watcher = trigger_owners(try_flatten_touched(watcher(api, lp)));
        self.selector.push(Box::pin(child_watcher));
        self
    }
    // TODO: fn watches and arbitrary stream?

    /// Consume the ControllerBuilder and create the controller
    fn run<ReconcilerFut, T>(
            self,
            mut reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
            mut error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> ReconcilerAction,
            context: Context<T>,
        ) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error, QueueStream::Error>>>
        where
            K: Clone + Meta + 'static,
            ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
            ReconcilerFut::Error: std::error::Error + 'static,
    {
        // TODO: how to get Items in this stream to have the same opaque type?
        let input_stream = stream::select_all(self.selector);
        controller(reconciler, error_policy, context, self.reader, input_stream)
    }

}
