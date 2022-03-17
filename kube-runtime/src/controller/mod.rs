//! Runs a user-supplied reconciler function on objects when they (or related objects) are updated

use self::runner::Runner;
use crate::{
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef,
    },
    scheduler::{scheduler, ScheduleRequest},
    utils::{
        try_flatten_applied, try_flatten_touched, trystream_try_via, CancelableJoinHandle,
        KubeRuntimeStreamExt, StreamBackoff,
    },
    watcher::{self, watcher},
};
use backoff::backoff::Backoff;
use derivative::Derivative;
use futures::{
    channel,
    future::{self, BoxFuture},
    stream, Future, FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt, TryStream, TryStreamExt,
};
use kube_client::api::{Api, DynamicObject, ListParams, Resource};
use serde::de::DeserializeOwned;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
    time::Duration,
};
use stream::BoxStream;
use thiserror::Error;
use tokio::{runtime::Handle, time::Instant};
use tracing::{info_span, Instrument};

mod future_hash_map;
mod runner;

#[derive(Debug, Error)]
pub enum Error<ReconcilerErr: std::error::Error + 'static, QueueErr: std::error::Error + 'static> {
    #[error("tried to reconcile object {0} that was not found in local store")]
    ObjectNotFound(ObjectRef<DynamicObject>),
    #[error("reconciler for object {1} failed")]
    ReconcilerFailed(#[source] ReconcilerErr, ObjectRef<DynamicObject>),
    #[error("event queue error")]
    QueueError(#[source] QueueErr),
}

/// Results of the reconciliation attempt
#[derive(Debug, Clone)]
pub struct Action {
    /// Whether (and when) to next trigger the reconciliation if no external watch triggers hit
    ///
    /// For example, use this to query external systems for updates, expire time-limited resources, or
    /// (in your `error_policy`) retry after errors.
    requeue_after: Option<Duration>,
}

impl Action {
    /// Action to to the reconciliation at this time even if no external watch triggers hit
    ///
    /// This is the best-practice action that ensures eventual consistency of your controller
    /// even in the case of missed changes (which can happen).
    ///
    /// Watch events are not normally missed, so running this once per hour (`Default`) as a fallback is reasonable.
    #[must_use]
    pub fn requeue(duration: Duration) -> Self {
        Self {
            requeue_after: Some(duration),
        }
    }

    /// Do nothing until a change is detected
    ///
    /// This stops the controller periodically reconciling this object until a relevant watch event
    /// was **detected**.
    ///
    /// **Warning**: If you have watch desyncs, it is possible to miss changes entirely.
    /// It is therefore not recommended to disable requeuing this way, unless you have
    /// frequent changes to the underlying object, or some other hook to retain eventual consistency.
    #[must_use]
    pub fn await_change() -> Self {
        Self { requeue_after: None }
    }
}

/// Helper for building custom trigger filters, see the implementations of [`trigger_self`] and [`trigger_owners`] for some examples.
pub fn trigger_with<T, K, I, S>(
    stream: S,
    mapper: impl Fn(T) -> I,
) -> impl Stream<Item = Result<ReconcileRequest<K>, S::Error>>
where
    S: TryStream<Ok = T>,
    I: IntoIterator,
    I::Item: Into<ReconcileRequest<K>>,
    K: Resource,
{
    stream
        .map_ok(move |obj| stream::iter(mapper(obj).into_iter().map(Into::into).map(Ok)))
        .try_flatten()
}

/// Enqueues the object itself for reconciliation
pub fn trigger_self<K, S>(
    stream: S,
    dyntype: K::DynamicType,
) -> impl Stream<Item = Result<ReconcileRequest<K>, S::Error>>
where
    S: TryStream<Ok = K>,
    K: Resource,
    K::DynamicType: Clone,
{
    trigger_with(stream, move |obj| {
        Some(ReconcileRequest {
            obj_ref: ObjectRef::from_obj_with(&obj, dyntype.clone()),
            reason: ReconcileReason::ObjectUpdated,
        })
    })
}

/// Enqueues any owners of type `KOwner` for reconciliation
pub fn trigger_owners<KOwner, S>(
    stream: S,
    owner_type: KOwner::DynamicType,
    child_type: <S::Ok as Resource>::DynamicType,
) -> impl Stream<Item = Result<ReconcileRequest<KOwner>, S::Error>>
where
    S: TryStream,
    S::Ok: Resource,
    <S::Ok as Resource>::DynamicType: Clone,
    KOwner: Resource,
    KOwner::DynamicType: Clone,
{
    trigger_with(stream, move |obj| {
        let meta = obj.meta().clone();
        let ns = meta.namespace;
        let owner_type = owner_type.clone();
        let child_ref = ObjectRef::from_obj_with(&obj, child_type.clone()).erase();
        meta.owner_references
            .into_iter()
            .flatten()
            .filter_map(move |owner| ObjectRef::from_owner_ref(ns.as_deref(), &owner, owner_type.clone()))
            .map(move |owner_ref| ReconcileRequest {
                obj_ref: owner_ref,
                reason: ReconcileReason::RelatedObjectUpdated {
                    obj_ref: Box::new(child_ref.clone()),
                },
            })
    })
}

/// A context data type that's passed through to the controllers callbacks
///
/// `Context` gets passed to both the `reconciler` and the `error_policy` callbacks,
/// allowing a read-only view of the world without creating a big nested lambda.
/// More or less the same as Actix's [`Data`](https://docs.rs/actix-web/3.x/actix_web/web/struct.Data.html).
#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Context<T>(Arc<T>);

impl<T> Context<T> {
    /// Create new `Context` instance.
    #[must_use]
    pub fn new(state: T) -> Context<T> {
        Context(Arc::new(state))
    }

    /// Get reference to inner controller data.
    #[must_use]
    pub fn get_ref(&self) -> &T {
        self.0.as_ref()
    }

    /// Convert to the internal `Arc<T>`.
    #[must_use]
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

/// A request to reconcile an object, annotated with why that request was made.
///
/// NOTE: The reason is ignored for comparison purposes. This means that, for example,
/// an object can only occupy one scheduler slot, even if it has been scheduled for multiple reasons.
/// In this case, only *the first* reason is stored.
#[derive(Derivative)]
#[derivative(
    Debug(bound = "K::DynamicType: Debug"),
    Clone(bound = "K::DynamicType: Clone"),
    PartialEq(bound = "K::DynamicType: PartialEq"),
    Eq(bound = "K::DynamicType: Eq"),
    Hash(bound = "K::DynamicType: Hash")
)]
pub struct ReconcileRequest<K: Resource> {
    pub obj_ref: ObjectRef<K>,
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    pub reason: ReconcileReason,
}

impl<K: Resource> From<ObjectRef<K>> for ReconcileRequest<K> {
    fn from(obj_ref: ObjectRef<K>) -> Self {
        ReconcileRequest {
            obj_ref,
            reason: ReconcileReason::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReconcileReason {
    Unknown,
    ObjectUpdated,
    RelatedObjectUpdated { obj_ref: Box<ObjectRef<DynamicObject>> },
    ReconcilerRequestedRetry,
    ErrorPolicyRequestedRetry,
    BulkReconcile,
    Custom { reason: String },
}

impl Display for ReconcileReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconcileReason::Unknown => f.write_str("unknown"),
            ReconcileReason::ObjectUpdated => f.write_str("object updated"),
            ReconcileReason::RelatedObjectUpdated { obj_ref: object } => {
                f.write_fmt(format_args!("related object updated: {}", object))
            }
            ReconcileReason::BulkReconcile => f.write_str("bulk reconcile requested"),
            ReconcileReason::ReconcilerRequestedRetry => f.write_str("reconciler requested retry"),
            ReconcileReason::ErrorPolicyRequestedRetry => f.write_str("error policy requested retry"),
            ReconcileReason::Custom { reason } => f.write_str(reason),
        }
    }
}

/// Apply a reconciler to an input stream, with a given retry policy
///
/// Takes a `store` parameter for the core objects, which should usually be updated by a [`reflector`].
///
/// The `queue` indicates which objects should be reconciled. For the core objects this will usually be
/// the [`reflector`] (piped through [`trigger_self`]). If your core objects own any subobjects then you
/// can also make them trigger reconciliations by [merging](`futures::stream::select`) the [`reflector`]
/// with a [`watcher`](watcher()) or [`reflector`](reflector()) for the subobject.
///
/// This is the "hard-mode" version of [`Controller`], which allows you some more customization
/// (such as triggering from arbitrary [`Stream`]s), at the cost of being a bit more verbose.
pub fn applier<K, QueueStream, ReconcilerFut, T>(
    mut reconciler: impl FnMut(Arc<K>, Context<T>) -> ReconcilerFut,
    mut error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> Action,
    context: Context<T>,
    store: Store<K>,
    queue: QueueStream,
) -> impl Stream<Item = Result<(ObjectRef<K>, Action), Error<ReconcilerFut::Error, QueueStream::Error>>>
where
    K: Clone + Resource + 'static,
    K::DynamicType: Debug + Eq + Hash + Clone + Unpin,
    ReconcilerFut: TryFuture<Ok = Action> + Unpin,
    ReconcilerFut::Error: std::error::Error + 'static,
    QueueStream: TryStream,
    QueueStream::Ok: Into<ReconcileRequest<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    let (scheduler_shutdown_tx, scheduler_shutdown_rx) = channel::oneshot::channel();
    let err_context = context.clone();
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ReconcileRequest<K>>>(100);
    // Create a stream of ObjectRefs that need to be reconciled
    trystream_try_via(
        // input: stream combining scheduled tasks and user specified inputs event
        Box::pin(stream::select(
            // 1. inputs from users queue stream
            queue.map_err(Error::QueueError).map_ok(|request| ScheduleRequest {
                message: request.into(),
                run_at: Instant::now() + Duration::from_millis(1),
            })
            .on_complete(async move {
                // On error: scheduler has already been shut down and there is nothing for us to do
                let _ = scheduler_shutdown_tx.send(());
                tracing::debug!("applier queue terminated, starting graceful shutdown")
            }),
            // 2. requests sent to scheduler_tx
            scheduler_rx
                .map(Ok)
                .take_until(scheduler_shutdown_rx)
                .on_complete(async { tracing::debug!("applier scheduler consumer terminated") }),
        )),
        // all the Oks from the select gets passed through the scheduler stream, and are then executed
        move |s| {
            Runner::new(scheduler(s), move |request| {
                let request = request.clone();
                match store.get(&request.obj_ref) {
                    Some(obj) => {
                        let reconciler_span = info_span!("reconciling object", "object.ref" = %request.obj_ref, object.reason = %request.reason);
                        reconciler_span.in_scope(|| reconciler(obj, context.clone()))
                        .into_future()
                        .instrument(reconciler_span.clone())
                        // Reconciler errors are OK from the applier's PoV, we need to apply the error policy
                        // to them separately
                        .map(|res| Ok((request.obj_ref, res, reconciler_span)))
                        .left_future()
                    },
                    None => future::err(
                        Error::ObjectNotFound(request.obj_ref.erase())
                    )
                    .right_future(),
                }
            })
            .on_complete(async { tracing::debug!("applier runner terminated") })
        },
    )
    .on_complete(async { tracing::debug!("applier runner-merge terminated") })
    // finally, for each completed reconcile call:
    .and_then(move |(obj_ref, reconciler_result, reconciler_span)| {
        let (Action { requeue_after }, requeue_reason) = match &reconciler_result {
            Ok(action) =>
                // do what user told us
                (action.clone(), ReconcileReason::ReconcilerRequestedRetry),
            Err(err) =>
                // reconciler fn call failed
                (reconciler_span.in_scope(|| error_policy(err, err_context.clone())), ReconcileReason::ErrorPolicyRequestedRetry),
        };
        let mut scheduler_tx = scheduler_tx.clone();
        async move {
            // Transmit the requeue request to the scheduler (picked up again at top)
            if let Some(delay) = requeue_after {
                // Failure to schedule item = in graceful shutdown mode, ignore
                let _ = scheduler_tx
                    .send(ScheduleRequest {
                        message: ReconcileRequest {obj_ref: obj_ref.clone(), reason: requeue_reason},
                        run_at: Instant::now() + delay,
                    })
                    .await;
            }
            match reconciler_result {
                Ok(action) => Ok((obj_ref, action)),
                Err(err) => Err(Error::ReconcilerFailed(err, obj_ref.erase()))
            }
        }
    })
    .on_complete(async { tracing::debug!("applier terminated") })
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
/// use kube::{
///   Client, CustomResource,
///   api::{Api, ListParams},
///   runtime::controller::{Context, Controller, Action}
/// };
/// use serde::{Deserialize, Serialize};
/// use tokio::time::Duration;
/// use futures::StreamExt;
/// use k8s_openapi::api::core::v1::ConfigMap;
/// use schemars::JsonSchema;
/// use std::sync::Arc;
/// use thiserror::Error;
///
/// #[derive(Debug, Error)]
/// enum Error {}
///
/// /// A custom resource
/// #[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
/// #[kube(group = "nullable.se", version = "v1", kind = "ConfigMapGenerator", namespaced)]
/// struct ConfigMapGeneratorSpec {
///     content: String,
/// }
///
/// /// The reconciler that will be called when either object change
/// async fn reconcile(g: Arc<ConfigMapGenerator>, _ctx: Context<()>) -> Result<Action, Error> {
///     // .. use api here to reconcile a child ConfigMap with ownerreferences
///     // see configmapgen_controller example for full info
///     Ok(Action::requeue(Duration::from_secs(300)))
/// }
/// /// an error handler that will be called when the reconciler fails
/// fn error_policy(_error: &Error, _ctx: Context<()>) -> Action {
///     Action::requeue(Duration::from_secs(60))
/// }
///
/// /// something to drive the controller
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::try_default().await?;
///     let context = Context::new(()); // bad empty context - put client in here
///     let cmgs = Api::<ConfigMapGenerator>::all(client.clone());
///     let cms = Api::<ConfigMap>::all(client.clone());
///     Controller::new(cmgs, ListParams::default())
///         .owns(cms, ListParams::default())
///         .run(reconcile, error_policy, context)
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
    K: Clone + Resource + Debug + 'static,
    K::DynamicType: Eq + Hash,
{
    // NB: Need to Unpin for stream::select_all
    trigger_selector: stream::SelectAll<BoxStream<'static, Result<ReconcileRequest<K>, watcher::Error>>>,
    trigger_backoff: Box<dyn Backoff + Send>,
    /// [`run`](crate::Controller::run) starts a graceful shutdown when any of these [`Future`]s complete,
    /// refusing to start any new reconciliations but letting any existing ones finish.
    graceful_shutdown_selector: Vec<BoxFuture<'static, ()>>,
    /// [`run`](crate::Controller::run) terminates immediately when any of these [`Future`]s complete,
    /// requesting that all running reconciliations be aborted.
    /// However, note that they *will* keep running until their next yield point (`.await`),
    /// blocking [`tokio::runtime::Runtime`] destruction (unless you follow up by calling [`std::process::exit`] after `run`).
    forceful_shutdown_selector: Vec<BoxFuture<'static, ()>>,
    dyntype: K::DynamicType,
    reader: Store<K>,
}

impl<K> Controller<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Eq + Hash + Clone,
{
    /// Create a Controller on a type `K`
    ///
    /// Takes an [`Api`] object that determines how the `Controller` listens for changes to the `K`.
    ///
    /// The [`ListParams`] controls to the possible subset of objects of `K` that you want to manage
    /// and receive reconcile events for.
    /// For the full set of objects `K` in the given `Api` scope, you can use [`ListParams::default`].
    #[must_use]
    pub fn new(owned_api: Api<K>, lp: ListParams) -> Self
    where
        K::DynamicType: Default,
    {
        Self::new_with(owned_api, lp, Default::default())
    }

    /// Create a Controller on a type `K`
    ///
    /// Takes an [`Api`] object that determines how the `Controller` listens for changes to the `K`.
    ///
    /// The [`ListParams`] lets you define a possible subset of objects of `K` that you want the [`Api`]
    /// to watch - in the Api's  configured scope - and receive reconcile events for.
    /// For the full set of objects `K` in the given `Api` scope, you can use [`ListParams::default`].
    ///
    /// This variant constructor is for [`dynamic`] types found through discovery. Prefer [`Controller::new`] for static types.
    ///
    /// [`ListParams`]: kube_client::api::ListParams
    /// [`Api`]: kube_client::Api
    /// [`dynamic`]: kube_client::core::dynamic
    /// [`ListParams::default`]: kube_client::api::ListParams::default
    pub fn new_with(owned_api: Api<K>, lp: ListParams, dyntype: K::DynamicType) -> Self {
        let writer = Writer::<K>::new(dyntype.clone());
        let reader = writer.as_reader();
        let mut trigger_selector = stream::SelectAll::new();
        let self_watcher = trigger_self(
            try_flatten_applied(reflector(writer, watcher(owned_api, lp))),
            dyntype.clone(),
        )
        .boxed();
        trigger_selector.push(self_watcher);
        Self {
            trigger_selector,
            trigger_backoff: Box::new(watcher::default_backoff()),
            graceful_shutdown_selector: vec![
                // Fallback future, ensuring that we never terminate if no additional futures are added to the selector
                future::pending().boxed(),
            ],
            forceful_shutdown_selector: vec![
                // Fallback future, ensuring that we never terminate if no additional futures are added to the selector
                future::pending().boxed(),
            ],
            dyntype,
            reader,
        }
    }

    /// Specify the backoff policy for "trigger" watches
    ///
    /// This includes the core watch, as well as auxilary watches introduced by [`Self::owns`] and [`Self::watches`].
    ///
    /// The [`default_backoff`](crate::watcher::default_backoff) follows client-go conventions,
    /// but can be overridden by calling this method.
    #[must_use]
    pub fn trigger_backoff(mut self, backoff: impl Backoff + Send + 'static) -> Self {
        self.trigger_backoff = Box::new(backoff);
        self
    }

    /// Retrieve a copy of the reader before starting the controller
    pub fn store(&self) -> Store<K> {
        self.reader.clone()
    }

    /// Specify `Child` objects which `K` owns and should be watched
    ///
    /// Takes an [`Api`] object that determines how the `Controller` listens for changes to the `Child`.
    /// All owned `Child` objects **must** contain an [`OwnerReference`] pointing back to a `K`.
    ///
    /// The [`ListParams`] refer to the possible subset of `Child` objects that you want the [`Api`]
    ///  to watch - in the Api's configured scope - and receive reconcile events for.
    /// To watch the full set of `Child` objects in the given `Api` scope, you can use [`ListParams::default`].
    ///
    /// [`OwnerReference`]: k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference
    #[must_use]
    pub fn owns<Child: Clone + Resource<DynamicType = ()> + DeserializeOwned + Debug + Send + 'static>(
        self,
        api: Api<Child>,
        lp: ListParams,
    ) -> Self {
        self.owns_with(api, (), lp)
    }

    /// Specify `Child` objects which `K` owns and should be watched
    ///
    /// Same as [`Controller::owns`], but accepts a `DynamicType` so it can be used with dynamic resources.
    #[must_use]
    pub fn owns_with<Child: Clone + Resource + DeserializeOwned + Debug + Send + 'static>(
        mut self,
        api: Api<Child>,
        dyntype: Child::DynamicType,
        lp: ListParams,
    ) -> Self
    where
        Child::DynamicType: Debug + Eq + Hash + Clone,
    {
        let child_watcher = trigger_owners(
            try_flatten_touched(watcher(api, lp)),
            self.dyntype.clone(),
            dyntype,
        );
        self.trigger_selector.push(child_watcher.boxed());
        self
    }

    /// Specify `Watched` object which `K` has a custom relation to and should be watched
    ///
    /// To define the `Watched` relation with `K`, you **must** define a custom relation mapper, which,
    /// when given a `Watched` object, returns an option or iterator of relevant `ObjectRef<K>` to reconcile.
    ///
    /// If the relation `K` has to `Watched` is that `K` owns `Watched`, consider using [`Controller::owns`].
    ///
    /// Takes an [`Api`] object that determines how the `Controller` listens for changes to the `Watched`.
    ///
    /// The [`ListParams`] refer to the possible subset of `Watched` objects that you want the [`Api`]
    /// to watch - in the Api's configured scope - and run through the custom mapper.
    /// To watch the full set of `Watched` objects in given the `Api` scope, you can use [`ListParams::default`].
    #[must_use]
    pub fn watches<
        Other: Clone + Resource<DynamicType = ()> + DeserializeOwned + Debug + Send + 'static,
        I: 'static + IntoIterator<Item = ObjectRef<K>>,
    >(
        self,
        api: Api<Other>,
        lp: ListParams,
        mapper: impl Fn(Other) -> I + Sync + Send + 'static,
    ) -> Self
    where
        I::IntoIter: Send,
    {
        self.watches_with(api, (), lp, mapper)
    }

    /// Specify `Watched` object which `K` has a custom relation to and should be watched
    ///
    /// Same as [`Controller::watches`], but accepts a `DynamicType` so it can be used with dynamic resources.
    #[must_use]
    pub fn watches_with<
        Other: Clone + Resource + DeserializeOwned + Debug + Send + 'static,
        I: 'static + IntoIterator<Item = ObjectRef<K>>,
    >(
        mut self,
        api: Api<Other>,
        dyntype: Other::DynamicType,
        lp: ListParams,
        mapper: impl Fn(Other) -> I + Sync + Send + 'static,
    ) -> Self
    where
        I::IntoIter: Send,
        Other::DynamicType: Clone,
    {
        let other_watcher = trigger_with(try_flatten_touched(watcher(api, lp)), move |obj| {
            let watched_obj_ref = ObjectRef::from_obj_with(&obj, dyntype.clone()).erase();
            mapper(obj)
                .into_iter()
                .map(move |mapped_obj_ref| ReconcileRequest {
                    obj_ref: mapped_obj_ref,
                    reason: ReconcileReason::RelatedObjectUpdated {
                        obj_ref: Box::new(watched_obj_ref.clone()),
                    },
                })
        });
        self.trigger_selector.push(other_watcher.boxed());
        self
    }

    /// Trigger a reconciliation for all managed objects whenever `trigger` emits a value
    ///
    /// For example, this can be used to reconcile all objects whenever the controller's configuration changes.
    ///
    /// To reconcile all objects when a new line is entered:
    ///
    /// ```
    /// # async {
    /// use futures::stream::StreamExt;
    /// use k8s_openapi::api::core::v1::ConfigMap;
    /// use kube::{
    ///     Client,
    ///     api::{ListParams, Api, ResourceExt},
    ///     runtime::{controller::{Context, Controller, Action}},
    /// };
    /// use std::{convert::Infallible, io::BufRead};
    /// let (mut reload_tx, reload_rx) = futures::channel::mpsc::channel(0);
    /// // Using a regular background thread since tokio::io::stdin() doesn't allow aborting reads,
    /// // and its worker prevents the Tokio runtime from shutting down.
    /// std::thread::spawn(move || {
    ///     for _ in std::io::BufReader::new(std::io::stdin()).lines() {
    ///         let _ = reload_tx.try_send(());
    ///     }
    /// });
    /// Controller::new(
    ///     Api::<ConfigMap>::all(Client::try_default().await.unwrap()),
    ///     ListParams::default(),
    /// )
    /// .reconcile_all_on(reload_rx.map(|_| ()))
    /// .run(
    ///     |o, _| async move {
    ///         println!("Reconciling {}", o.name());
    ///         Ok(Action::await_change())
    ///     },
    ///     |err: &Infallible, _| Err(err).unwrap(),
    ///     Context::new(()),
    /// );
    /// # };
    /// ```
    ///
    /// This can be called multiple times, in which case they are additive; reconciles are scheduled whenever *any* [`Stream`] emits a new item.
    ///
    /// If a [`Stream`] is terminated (by emitting [`None`]) then the [`Controller`] keeps running, but the [`Stream`] stops being polled.
    #[must_use]
    pub fn reconcile_all_on(mut self, trigger: impl Stream<Item = ()> + Send + Sync + 'static) -> Self {
        let store = self.store();
        let dyntype = self.dyntype.clone();
        self.trigger_selector.push(
            trigger
                .flat_map(move |()| {
                    let dyntype = dyntype.clone();
                    stream::iter(store.state().into_iter().map(move |obj| {
                        Ok(ReconcileRequest {
                            obj_ref: ObjectRef::from_obj_with(&*obj, dyntype.clone()),
                            reason: ReconcileReason::BulkReconcile,
                        })
                    }))
                })
                .boxed(),
        );
        self
    }

    /// Start a graceful shutdown when `trigger` resolves. Once a graceful shutdown has been initiated:
    ///
    /// - No new reconciliations are started from the scheduler
    /// - The underlying Kubernetes watch is terminated
    /// - All running reconciliations are allowed to finish
    /// - [`Controller::run`]'s [`Stream`] terminates once all running reconciliations are done.
    ///
    /// For example, to stop the reconciler whenever the user presses Ctrl+C:
    ///
    /// ```rust
    /// # async {
    /// use futures::future::FutureExt;
    /// use k8s_openapi::api::core::v1::ConfigMap;
    /// use kube::{api::ListParams, Api, Client, ResourceExt};
    /// use kube_runtime::controller::{Context, Controller, Action};
    /// use std::convert::Infallible;
    /// Controller::new(
    ///     Api::<ConfigMap>::all(Client::try_default().await.unwrap()),
    ///     ListParams::default(),
    /// )
    /// .graceful_shutdown_on(tokio::signal::ctrl_c().map(|_| ()))
    /// .run(
    ///     |o, _| async move {
    ///         println!("Reconciling {}", o.name());
    ///         Ok(Action::await_change())
    ///     },
    ///     |err: &Infallible, _| Err(err).unwrap(),
    ///     Context::new(()),
    /// );
    /// # };
    /// ```
    ///
    /// This can be called multiple times, in which case they are additive; the [`Controller`] starts to terminate
    /// as soon as *any* [`Future`] resolves.
    #[must_use]
    pub fn graceful_shutdown_on(mut self, trigger: impl Future<Output = ()> + Send + Sync + 'static) -> Self {
        self.graceful_shutdown_selector.push(trigger.boxed());
        self
    }

    /// Initiate graceful shutdown on Ctrl+C or SIGTERM (on Unix), waiting for all reconcilers to finish.
    ///
    /// Once a graceful shutdown has been initiated, Ctrl+C (or SIGTERM) can be sent again
    /// to request a forceful shutdown (requesting that all reconcilers abort on the next yield point).
    ///
    /// NOTE: On Unix this leaves the default handlers for SIGINT and SIGTERM disabled after the [`Controller`] has
    /// terminated. If you run this in a process containing more tasks than just the [`Controller`], ensure that
    /// all other tasks either terminate when the [`Controller`] does, that they have their own signal handlers,
    /// or use [`Controller::graceful_shutdown_on`] to manage your own shutdown strategy.
    ///
    /// NOTE: If developing a Windows service then you need to listen to its lifecycle events instead, and hook that into
    /// [`Controller::graceful_shutdown_on`].
    ///
    /// NOTE: [`Controller::run`] terminates as soon as a forceful shutdown is requested, but leaves the reconcilers running
    /// in the background while they terminate. This will block [`tokio::runtime::Runtime`] termination until they actually terminate,
    /// unless you run [`std::process::exit`] afterwards.
    #[must_use]
    pub fn shutdown_on_signal(mut self) -> Self {
        async fn shutdown_signal() {
            futures::future::select(
                tokio::signal::ctrl_c().map(|_| ()).boxed(),
                #[cfg(unix)]
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .unwrap()
                    .recv()
                    .map(|_| ())
                    .boxed(),
                // Assume that ctrl_c is enough on non-Unix platforms (such as Windows)
                #[cfg(not(unix))]
                futures::future::pending::<()>(),
            )
            .await;
        }

        let (graceful_tx, graceful_rx) = channel::oneshot::channel();
        self.graceful_shutdown_selector
            .push(graceful_rx.map(|_| ()).boxed());
        self.forceful_shutdown_selector.push(
            async {
                tracing::info!("press ctrl+c to shut down gracefully");
                shutdown_signal().await;
                if let Ok(()) = graceful_tx.send(()) {
                    tracing::info!("graceful shutdown requested, press ctrl+c again to force shutdown");
                } else {
                    tracing::info!(
                        "graceful shutdown already requested, press ctrl+c again to force shutdown"
                    );
                }
                shutdown_signal().await;
                tracing::info!("forced shutdown requested");
            }
            .boxed(),
        );
        self
    }

    /// Consume all the parameters of the Controller and start the applier stream
    ///
    /// This creates a stream from all builder calls and starts an applier with
    /// a specified `reconciler` and `error_policy` callbacks. Each of these will be called
    /// with a configurable [`Context`].
    pub fn run<ReconcilerFut, T>(
        self,
        mut reconciler: impl FnMut(Arc<K>, Context<T>) -> ReconcilerFut,
        error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> Action,
        context: Context<T>,
    ) -> impl Stream<Item = Result<(ObjectRef<K>, Action), Error<ReconcilerFut::Error, watcher::Error>>>
    where
        K::DynamicType: Debug + Unpin,
        ReconcilerFut: TryFuture<Ok = Action> + Send + 'static,
        ReconcilerFut::Error: std::error::Error + Send + 'static,
    {
        applier(
            move |obj, ctx| {
                CancelableJoinHandle::spawn(
                    reconciler(obj, ctx).into_future().in_current_span(),
                    &Handle::current(),
                )
            },
            error_policy,
            context,
            self.reader,
            StreamBackoff::new(self.trigger_selector, self.trigger_backoff)
                .take_until(future::select_all(self.graceful_shutdown_selector)),
        )
        .take_until(futures::future::select_all(self.forceful_shutdown_selector))
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Context};
    use crate::Controller;
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube_client::Api;

    fn assert_send<T: Send>(x: T) -> T {
        x
    }

    fn mock_type<T>() -> T {
        unimplemented!(
            "mock_type is not supposed to be called, only used for filling holes in type assertions"
        )
    }

    // not #[test] because we don't want to actually run it, we just want to assert that it typechecks
    #[allow(dead_code, unused_must_use)]
    fn test_controller_should_be_send() {
        assert_send(
            Controller::new(mock_type::<Api<ConfigMap>>(), Default::default()).run(
                |_, _| async { Ok(mock_type::<Action>()) },
                |_: &std::io::Error, _| mock_type::<Action>(),
                Context::new(()),
            ),
        );
    }
}
