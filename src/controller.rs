use crate::{
    reflector::{Cache, ErasedResource, ObjectRef},
    scheduler::{self, scheduler, ScheduleRequest},
};
use futures::{
    channel, future, stream, FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt,
    TryStreamExt,
};
use kube::api::Meta;
use snafu::{futures::TryStreamExt as SnafuTryStreamExt, Backtrace, OptionExt, ResultExt, Snafu};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Snafu, Debug)]
pub enum Error<ReconcilerErr: std::error::Error + 'static> {
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
}

#[derive(Debug, Clone)]
pub struct ReconcilerAction {
    requeue_after: Option<Duration>,
}

/// Enqueues the object itself for reconciliation
pub fn trigger_self<K: Meta>(stream: impl Stream<Item = K>) -> impl Stream<Item = ObjectRef<K>> {
    stream.map(|obj| ObjectRef::from_obj(&obj))
}

/// Runs a reconciler whenever an object changes
///
/// The `store` should be kept updated by a `reflector`.
///
/// The `queue` is a source of external events that trigger the reconciler,
/// usually taken from a `reflector` and then passed through a trigger function such as
/// `trigger_self`.
pub fn controller<K, ReconcilerFut>(
    mut reconciler: impl FnMut(K) -> ReconcilerFut,
    mut error_policy: impl FnMut(&ReconcilerFut::Error) -> ReconcilerAction,
    store: Cache<K>,
    queue: impl Stream<Item = ObjectRef<K>>,
) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error>>>
where
    K: Clone + Meta + 'static,
    ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
    ReconcilerFut::Error: std::error::Error + 'static,
{
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ObjectRef<K>>>(100);
    let scheduler_rx = scheduler(scheduler_rx);
    stream::select(queue.map(Ok), scheduler_rx.context(SchedulerDequeueFailed))
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
