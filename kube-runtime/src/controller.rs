use crate::{
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef, RuntimeResource,
    },
    scheduler::{self, scheduler, ScheduleRequest},
    utils::{try_flatten_applied, try_flatten_touched, trystream_try_via},
    watcher::{self, watcher},
};
use backoff::backoff::Backoff;
use derivative::Derivative;
use futures::{
    channel, future,
    stream::{self, SelectAll},
    FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt, TryStream, TryStreamExt,
};
use kube::api::{Api, ListParams, Meta};
use serde::de::DeserializeOwned;
use snafu::{futures::TryStreamExt as SnafuTryStreamExt, Backtrace, OptionExt, ResultExt, Snafu};
use std::{collections::HashMap, fmt::Debug, marker::PhantomData, sync::Arc, time::Duration};
use stream::BoxStream;
use tokio::time::Instant;

#[derive(Snafu, Derivative)]
#[derivative(Debug(bound = ""))]
pub enum Error<
    K: RuntimeResource,
    ReconcilerErr: std::error::Error + 'static,
    QueueErr: std::error::Error + 'static,
> {
    ObjectNotFound {
        obj_ref: ObjectRef<K>,
        backtrace: Backtrace,
    },
    ReconcilerFailed {
        obj_ref: ObjectRef<K>,
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

/// Results of the reconciliation attempt
#[derive(Debug, Clone)]
pub struct ReconcilerAction {
    /// Whether (and when) to next trigger the reconciliation if no external watch triggers hit
    ///
    /// For example, use this to query external systems for updates, expire time-limited resources, or
    /// (in your `error_policy`) retry after errors.
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

/// A policy for when to retry reconciliation, after an error has occurred.
pub trait ErrorPolicy {
    type Err;
    type K: RuntimeResource;
    type Ctx;

    /// Notifies that the state for an object should be removed, for example if it is reconciled successfully.
    fn reset_object(&mut self, obj_ref: &ObjectRef<Self::K>, ctx: Context<Self::Ctx>);
    /// Queries for when to next retry after an error.
    fn on_error(
        &mut self,
        obj_ref: ObjectRef<Self::K>,
        error: &Self::Err,
        ctx: Context<Self::Ctx>,
    ) -> ReconcilerAction;
}

/// Retries errors based on a `Backoff` policy.
///
/// A separate backoff tracker is used for each object, and it is
/// reset whenever the object is reconciled successfully.
#[derive(Debug)]
pub struct BackoffErrorPolicy<MkBackoff, B, K: RuntimeResource, Err, Ctx> {
    make_backoff: MkBackoff,
    backoffs: HashMap<ObjectRef<K>, B>,
    _err: PhantomData<Err>,
    _ctx: PhantomData<Ctx>,
}

impl<MkBackoff: FnMut() -> B, B: Backoff, K: RuntimeResource, Err, Ctx>
    BackoffErrorPolicy<MkBackoff, B, K, Err, Ctx>
{
    fn new(make_backoff: MkBackoff) -> Self {
        BackoffErrorPolicy {
            make_backoff,
            backoffs: HashMap::new(),
            _err: PhantomData,
            _ctx: PhantomData,
        }
    }
}

impl<MkBackoff: FnMut() -> B, B: Backoff, K: RuntimeResource, Err, Ctx> ErrorPolicy
    for BackoffErrorPolicy<MkBackoff, B, K, Err, Ctx>
{
    type Err = Err;
    type K = K;
    type Ctx = Ctx;

    fn reset_object(&mut self, obj_ref: &ObjectRef<Self::K>, _ctx: Context<Self::Ctx>) {
        self.backoffs.remove(obj_ref);
    }

    fn on_error(
        &mut self,
        obj_ref: ObjectRef<Self::K>,
        _error: &Self::Err,
        _ctx: Context<Self::Ctx>,
    ) -> ReconcilerAction {
        let obj_backoff = self
            .backoffs
            .entry(obj_ref)
            .or_insert_with(&mut self.make_backoff);
        ReconcilerAction {
            requeue_after: obj_backoff.next_backoff(),
        }
    }
}

/// A context data type that's passed through to the controllers callbacks
///
/// Context<T> gets passed to both the `reconciler` and the `error_policy` callbacks.
/// allowing a read-only view of the world without creating a big nested lambda.
/// More or less the same as actix's Data<T>
#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Context<T>(Arc<T>);

impl<T> Context<T> {
    /// Create new `Data` instance.
    #[must_use]
    pub fn new(state: T) -> Context<T> {
        Context(Arc::new(state))
    }

    /// Get reference to inner controller data.
    #[must_use]
    pub fn get_ref(&self) -> &T {
        self.0.as_ref()
    }

    /// Convert to the internal Arc<T>
    #[must_use]
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

/// Apply a reconciler to an input stream, with a given retry policy
///
/// Takes a `store` parameter for the main object which should be updated by a `reflector`.
///
/// The `queue` is a source of external events that trigger the reconciler,
/// usually taken from a `reflector` and then passed through a trigger function such as
/// `trigger_self`.
///
/// This is the "hard-mode" version of `Controller`, which allows you some more customization
/// (such as triggering from arbitrary `Stream`s), at the cost of some more verbosity.
pub fn applier<K, QueueStream, ReconcilerFut, T>(
    mut reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
    mut error_policy: impl ErrorPolicy<K = K, Err = ReconcilerFut::Error, Ctx = T>,
    context: Context<T>,
    store: Store<K>,
    queue: QueueStream,
) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<K, ReconcilerFut::Error, QueueStream::Error>>>
where
    K: Clone + Meta + 'static,
    ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
    ReconcilerFut::Error: std::error::Error + 'static,
    QueueStream: TryStream<Ok = ObjectRef<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    let err_context = context.clone();
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ObjectRef<K>>>(100);
    // Create a stream of ObjectRefs that need to be reconciled
    trystream_try_via(
        // input: stream combining scheduled tasks and user specified inputs event
        Box::pin(stream::select(
            // 1. inputs from users queue stream
            queue.context(QueueError).map_ok(|obj_ref| ScheduleRequest {
                message: obj_ref,
                run_at: Instant::now() + Duration::from_millis(1),
            }),
            // 2. requests sent to scheduler_tx
            scheduler_rx.map(Ok),
        )),
        // all the Oks from the select gets passed through the scheduler stream
        |s| scheduler(s).context(SchedulerDequeueFailed),
    )
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
        reconciler(obj, context.clone())
            .into_future() // TryFuture -> impl Future
            .map(|result| match result {
                Ok(action) => Ok((obj_ref, action)),
                Err(err) => Err(err).context(ReconcilerFailed { obj_ref }),
            })
    })
    // finally, for each completed reconcile call:
    .then(move |reconciler_result| {
        let (obj_ref, action, error) = match reconciler_result {
            // tell the error policy about the success (to reset backoff timers, for example)
            Ok((obj_ref, action)) => {
                error_policy.reset_object(&obj_ref, err_context.clone());
                (obj_ref.clone(), action, None)
            }
            // reconciler fn call failed
            Err(Error::ReconcilerFailed {
                obj_ref,
                source,
                backtrace,
            }) => (
                obj_ref.clone(),
                error_policy.on_error(obj_ref.clone(), &source, err_context.clone()),
                Some(Error::ReconcilerFailed {
                    obj_ref,
                    source,
                    backtrace,
                }),
            ),
            // object was deleted, fake a "success" to the error policy, so that it can clean up any bookkeeping and avoid leaking memory
            Err(Error::ObjectNotFound { obj_ref, backtrace }) => {
                error_policy.reset_object(&obj_ref, err_context.clone());
                (
                    obj_ref.clone(),
                    ReconcilerAction { requeue_after: None },
                    Some(Error::ObjectNotFound { obj_ref, backtrace }),
                )
            }
            // Upstream or internal error, propagate
            Err(_) => return future::Either::Left(future::ready(reconciler_result)),
        };
        let mut scheduler_tx = scheduler_tx.clone();
        future::Either::Right(async move {
            // Transmit the requeue request to the scheduler (picked up again at top)
            if let Some(delay) = action.requeue_after {
                scheduler_tx
                    .send(ScheduleRequest {
                        message: obj_ref.clone(),
                        run_at: Instant::now() + delay,
                    })
                    .await
                    .expect("Message could not be sent to scheduler_rx");
            }
            match error {
                None => Ok((obj_ref, action)),
                Some(err) => Err(err),
            }
        })
    })
}

/// Controller
///
/// A controller is made up of:
/// - 1 `reflector` (for the core object)
/// - N `watcher` objects for each object child object
/// - user defined `reconcile` + `error_policy` callbacks
/// - a generated input stream considering all sources
///
/// And all reconcile requests  through an internal scheduler
///
/// Pieces:
/// ```no_run
/// use kube::{Client, api::{Api, ListParams}};
/// use kube_derive::CustomResource;
/// use serde::{Deserialize, Serialize};
/// use tokio::time::Duration;
/// use futures::StreamExt;
/// use kube_runtime::controller::{Context, Controller, ReconcilerAction};
/// use k8s_openapi::api::core::v1::ConfigMap;
///
/// use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
/// #[derive(Debug, Snafu)]
/// enum Error {}
/// /// A custom resource
/// #[derive(CustomResource, Debug, Clone, Deserialize, Serialize)]
/// #[kube(group = "nullable.se", version = "v1", namespaced)]
/// struct ConfigMapGeneratorSpec {
///     content: String,
/// }
///
/// /// The reconciler that will be called when either object change
/// async fn reconcile(g: ConfigMapGenerator, _ctx: Context<()>) -> Result<ReconcilerAction, Error> {
///     // .. use api here to reconcile a child ConfigMap with ownerreferences
///     // see configmapgen_controller example for full info
///     Ok(ReconcilerAction {
///         requeue_after: Some(Duration::from_secs(300)),
///     })
/// }
///
/// /// something to drive the controller
/// #[tokio::main]
/// async fn main() -> Result<(), kube::Error> {
///     let client = Client::try_default().await?;
///     let context = Context::new(()); // bad empty context - put client in here
///     let cmgs = Api::<ConfigMapGenerator>::all(client.clone());
///     let cms = Api::<ConfigMap>::all(client.clone());
///     Controller::new(cmgs, ListParams::default())
///         .owns(cms, ListParams::default())
///         .run(reconcile, || backoff::backoff::Constant::new(Duration::from_secs(60)), context)
///         .for_each(|res| async move {
///             match res {
///                 Ok(o) => println!("reconciled {:?}", o),
///                 Err(e) => println!("reconcile failed: {:?}", e),
///             }
///         })
///         .await; // controller does nothing unless polled
///     Ok(())
/// }
/// ```
pub struct Controller<K>
where
    K: Clone + Meta + 'static,
{
    // NB: Need to Unpin for stream::select_all
    // TODO: get an arbitrary std::error::Error in here?
    selector: SelectAll<BoxStream<'static, Result<ObjectRef<K>, watcher::Error>>>,
    reader: Store<K>,
}

impl<K> Controller<K>
where
    K: Clone + Meta + DeserializeOwned + Send + Sync + 'static,
{
    /// Create a Controller on a type `K`
    ///
    /// Configure `ListParams` and `Api` so you only get reconcile events
    /// for the correct `Api` scope (cluster/all/namespaced), or `ListParams` subset
    pub fn new(owned_api: Api<K>, lp: ListParams) -> Self {
        let writer = Writer::<K>::default();
        let reader = writer.as_reader();
        let mut selector = stream::SelectAll::new();
        let self_watcher =
            trigger_self(try_flatten_applied(reflector(writer, watcher(owned_api, lp)))).boxed();
        selector.push(self_watcher);
        Self { selector, reader }
    }

    /// Retrieve a copy of the reader before starting the controller
    pub fn store(&self) -> Store<K> {
        self.reader.clone()
    }

    /// Indicate child objets `K` owns and be notified when they change
    ///
    /// This type `Child` must have `OwnerReference`s set to point back to `K`.
    /// You can customize the parameters used by the underlying `watcher` if
    /// only a subset of `Child` entries are required.
    /// The `api` must have the correct scope (cluster/all namespaces, or namespaced)
    pub fn owns<Child: Clone + Meta + DeserializeOwned + Send + 'static>(
        mut self,
        api: Api<Child>,
        lp: ListParams,
    ) -> Self {
        let child_watcher = trigger_owners(try_flatten_touched(watcher(api, lp)));
        self.selector.push(child_watcher.boxed());
        self
    }

    /// Indicate an object to watch with a custom mapper
    ///
    /// This mapper should return something like Option<ObjectRef<K>>
    pub fn watches<
        Other: Clone + Meta + DeserializeOwned + Send + 'static,
        I: 'static + IntoIterator<Item = ObjectRef<K>>,
    >(
        mut self,
        api: Api<Other>,
        lp: ListParams,
        mapper: impl Fn(Other) -> I + Send + 'static,
    ) -> Self
    where
        I::IntoIter: Send,
    {
        let other_watcher = trigger_with(try_flatten_touched(watcher(api, lp)), mapper);
        self.selector.push(other_watcher.boxed());
        self
    }

    /// Consume all the parameters of the Controller and start the applier stream
    ///
    /// This creates a stream from all builder calls and starts an applier with
    /// a specified `reconciler` and `error_policy` callbacks. Each of these will be called
    /// with a configurable `Context`.
    pub fn run<ReconcilerFut, T, B>(
        self,
        reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
        make_backoff: impl Fn() -> B,
        context: Context<T>,
    ) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<K, ReconcilerFut::Error, watcher::Error>>>
    where
        K: Clone + Meta + 'static,
        ReconcilerFut: TryFuture<Ok = ReconcilerAction>,
        ReconcilerFut::Error: std::error::Error + 'static,
        B: Backoff,
    {
        applier(
            reconciler,
            BackoffErrorPolicy::new(make_backoff),
            context,
            self.reader,
            self.selector,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, ReconcilerAction};
    use crate::Controller;
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::Api;
    use snafu::Snafu;

    fn assert_send<T: Send>(x: T) -> T {
        x
    }

    fn mock_type<T>() -> T {
        unimplemented!(
            "mock_type is not supposed to be called, only used for filling holes in type assertions"
        )
    }

    #[derive(Snafu, Debug)]
    enum NoError {}

    // not #[test] because we don't want to actually run it, we just want to assert that it typechecks
    #[allow(dead_code, unused_must_use)]
    fn test_controller_should_be_send() {
        assert_send(
            Controller::new(mock_type::<Api<ConfigMap>>(), Default::default()).run(
                |_, _| async { Ok::<_, NoError>(mock_type::<ReconcilerAction>()) },
                || backoff::backoff::Zero {},
                Context::new(()),
            ),
        );
    }
}
