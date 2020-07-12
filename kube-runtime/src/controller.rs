use crate::{
    reflector::{ErasedResource, ObjectRef, Store},
    scheduler::{self, scheduler, ScheduleRequest},
    utils::trystream_try_via,
};
use futures::{
    channel, future, stream, FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt, TryStream,
    TryStreamExt,
};
use kube::api::Meta;
use snafu::{futures::TryStreamExt as SnafuTryStreamExt, Backtrace, OptionExt, ResultExt, Snafu};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Snafu, Debug)]
pub enum Error<ReconcilerErr: std::error::Error + 'static, QueueErr: std::error::Error + 'static> {
    ObjectNotFound {
        obj_ref: ObjectRef<ErasedResource>,
        backtrace: Backtrace,
    },
    ReconcilerFailed {
        source: ReconcilerErr,
        backtrace: Backtrace,
    },
    SchedulerDequeueFailed {
        #[snafu(backtrace)]
        source: scheduler::Error,
    },
    QueueError {
        source: QueueErr,
        backtrace: Backtrace,
    },
}

#[derive(Debug, Clone)]
pub struct ReconcilerAction {
    pub requeue_after: Option<Duration>,
}

/// Helper for building custom trigger filters, see `trigger_self` and `trigger_owners` for some examples
pub fn trigger_with<T, K, I, S>(
    stream: S,
    mapper: impl Fn(T) -> I,
) -> impl Stream<Item = Result<ObjectRef<K>, S::Error>>
where
    S: TryStream<Ok = T>,
    I: IntoIterator<Item = ObjectRef<K>>,
    K: Meta,
{
    stream
        .map_ok(move |obj| stream::iter(mapper(obj).into_iter().map(Ok)))
        .try_flatten()
}

/// Enqueues the object itself for reconciliation
pub fn trigger_self<S>(stream: S) -> impl Stream<Item = Result<ObjectRef<S::Ok>, S::Error>>
where
    S: TryStream,
    S::Ok: Meta,
{
    trigger_with(stream, |obj| Some(ObjectRef::from_obj(&obj)))
}

/// Enqueues any owners of type `KOwner` for reconciliation
pub fn trigger_owners<KOwner, S>(stream: S) -> impl Stream<Item = Result<ObjectRef<KOwner>, S::Error>>
where
    S: TryStream,
    S::Ok: Meta,
    KOwner: Meta,
{
    trigger_with(stream, |obj| {
        let meta = obj.meta().clone();
        let ns = meta.namespace;
        meta.owner_references
            .into_iter()
            .flatten()
            .flat_map(move |owner| ObjectRef::from_owner_ref(ns.as_deref(), &owner))
    })
}

/// Runs a reconciler whenever an input stream change
///
/// Takes a `store` parameter for the main object which should be updated by a `reflector`.
///
/// The `queue` is a source of external events that trigger the reconciler,
/// usually taken from a `reflector` and then passed through a trigger function such as
/// `trigger_self`.
pub fn controller<K, QueueStream, ReconcilerFut>(
    mut reconciler: impl FnMut(K) -> ReconcilerFut,
    mut error_policy: impl FnMut(&ReconcilerFut::Error) -> ReconcilerAction,
    store: Store<K>,
    queue: QueueStream,
) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error, QueueStream::Error>>>
where
    K: Clone + Meta + 'static,
    ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
    ReconcilerFut::Error: std::error::Error + 'static,
    QueueStream: TryStream<Ok = ObjectRef<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ObjectRef<K>>>(100);
    // Create a stream of ObjectRefs that need to be reconciled
    trystream_try_via(
        // input: stream combining scheduled tasks and user specified inputs event
        Box::pin(stream::select(
            // user inputs
            queue.context(QueueError).map_ok(|obj_ref| ScheduleRequest {
                message: obj_ref,
                run_at: Instant::now() + Duration::from_millis(1),
            }),
            // back from scheduler
            scheduler_rx.map(Ok),
        )),
        // all the Oks from the select gets passed through the scheduler stream
        |s| scheduler(s).context(SchedulerDequeueFailed),
    )
    // TODO: maybe we could do stream::select -> stream::split / stream::map_ok to bypass `trystream_try_via`
    // now have ObjectRefs that we turn into pairs inside (no extra waiting introduced)
    .and_then(move |obj_ref| {
        future::ready(
            store
                .get(&obj_ref)
                .context(ObjectNotFound {
                    obj_ref: obj_ref.clone(),
                })
                .map(|obj| (obj_ref, obj)),
        )
    })
    // then reconcile every object
    .and_then(move |(obj_ref, obj)| {
        reconciler(obj) // TODO: add a context argument to the reconcile
            .into_future() // TryFuture -> impl Future
            .map(|result| (obj_ref, result)) // turn into pair and ok wrap
            .map(Ok) // (this lets us deal with errors from reconciler below)
    })
    // finally, for each completed reconcile call:
    .and_then(move |(obj_ref, reconciler_result)| {
        let ReconcilerAction { requeue_after } = match &reconciler_result {
            Ok(action) => action.clone(),  // do what user told us
            Err(err) => error_policy(err), // reconciler fn call failed
        };
        // we should always requeue at some point in case of network errors ^
        let mut scheduler_tx = scheduler_tx.clone();
        async move {
            // Transmit the requeue request to the scheduler (picked up again at top)
            if let Some(delay) = requeue_after {
                scheduler_tx
                    .send(ScheduleRequest {
                        message: obj_ref.clone(),
                        run_at: Instant::now() + delay,
                    })
                    .await
                    .expect("Message could not be sent to scheduler_rx");
            }
            // NB: no else clause ^ because we don't allow not requeuing atm.
            reconciler_result
                .map(|action| (obj_ref, action))
                .context(ReconcilerFailed)
        }
    })
}
