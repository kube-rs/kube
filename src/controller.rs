use crate::{
    reflector::{Cache, ErasedResource, ObjectRef},
    scheduler::{self, scheduler, ScheduleRequest},
};
use futures::{
    channel, future, stream, FutureExt, SinkExt, Stream, TryFuture, TryFutureExt, TryStream,
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

/// Enqueues the object itself for reconciliation
pub fn trigger_self<S>(stream: S) -> impl Stream<Item = Result<ObjectRef<S::Ok>, S::Error>>
where
    S: TryStream,
    S::Ok: Meta,
{
    stream.map_ok(|obj| ObjectRef::from_obj(&obj))
}

/// Enqueues any owners of type `KOwner` for reconciliation
pub fn trigger_owners<KOwner, S>(
    stream: S,
) -> impl Stream<Item = Result<ObjectRef<KOwner>, S::Error>>
where
    S: TryStream,
    S::Ok: Meta,
    KOwner: Meta,
{
    stream
        .map_ok(|obj| {
            let meta = obj.meta().clone();
            let ns = meta.namespace;
            stream::iter(
                meta.owner_references
                    .into_iter()
                    .flatten()
                    .map(move |owner| ObjectRef::from_owner_ref(ns.as_deref(), &owner))
                    .flatten()
                    .map(Ok),
            )
        })
        .try_flatten()
}

/// Runs a reconciler whenever an object changes
///
/// The `store` should be kept updated by a `reflector`.
///
/// The `queue` is a source of external events that trigger the reconciler,
/// usually taken from a `reflector` and then passed through a trigger function such as
/// `trigger_self`.
pub fn controller<K, QueueStream, ReconcilerFut>(
    mut reconciler: impl FnMut(K) -> ReconcilerFut,
    mut error_policy: impl FnMut(&ReconcilerFut::Error) -> ReconcilerAction,
    store: Cache<K>,
    queue: QueueStream,
) -> impl Stream<
    Item = Result<
        (ObjectRef<K>, ReconcilerAction),
        Error<ReconcilerFut::Error, QueueStream::Error>,
    >,
>
where
    K: Clone + Meta + 'static,
    ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
    ReconcilerFut::Error: std::error::Error + 'static,
    QueueStream: TryStream<Ok = ObjectRef<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ObjectRef<K>>>(100);
    let scheduler_rx = scheduler(scheduler_rx);
    stream::select(
        queue.context(QueueError),
        scheduler_rx.context(SchedulerDequeueFailed),
    )
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
    .and_then(move |(obj_ref, obj)| {
        reconciler(obj)
            .into_future()
            .map(|result| (obj_ref, result))
            .map(Ok)
    })
    .and_then(move |(obj_ref, reconciler_result)| {
        let ReconcilerAction { requeue_after } = match &reconciler_result {
            Ok(action) => action.clone(),
            Err(err) => error_policy(err),
        };
        let mut scheduler_tx = scheduler_tx.clone();
        async move {
            if let Some(delay) = requeue_after {
                scheduler_tx
                    .send(ScheduleRequest {
                        message: obj_ref.clone(),
                        run_at: Instant::now() + delay,
                    })
                    .await
                    .unwrap();
            }
            reconciler_result
                .map(|action| (obj_ref, action))
                .context(ReconcilerFailed)
        }
    })
}
