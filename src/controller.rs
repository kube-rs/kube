use crate::reflector::{Cache, ErasedResource, ObjectRef};
use futures::Stream;
use futures::{FutureExt, StreamExt, TryFuture, TryFutureExt, TryStreamExt};
use kube::api::Meta;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use std::time::Duration;

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
}

#[derive(Debug, Clone)]
pub struct ReconcilerAction {
    requeue_after: Option<Duration>,
}

pub fn enqueue_self<K: Meta>(stream: impl Stream<Item = K>) -> impl Stream<Item = ObjectRef<K>> {
    stream.map(|obj| ObjectRef::from_obj(&obj))
}

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
    queue
        .map(move |obj_ref| {
            store
                .get(&obj_ref)
                .context(ObjectNotFound {
                    obj_ref: obj_ref.clone(),
                })
                .map(|obj| (obj_ref, obj))
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
            if let Some(delay) = requeue_after {
                todo!("requeue {:?} after {:?}", obj_ref, delay);
            }
            async {
                reconciler_result
                    .map(|action| (obj_ref, action))
                    .context(ReconcilerFailed)
            }
        })
}
