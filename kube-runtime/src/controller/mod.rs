//! Runs a user-supplied reconciler function on objects when they (or related objects) are updated

use self::runner::Runner;
use crate::{
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef,
    },
    scheduler::{self, scheduler, ScheduleRequest},
    utils::{try_flatten_applied, try_flatten_touched, trystream_try_via, CancelableJoinHandle},
    watcher::{self, watcher},
};
use derivative::Derivative;
use futures::{
    channel, future, stream, FutureExt, SinkExt, Stream, StreamExt, TryFuture, TryFutureExt, TryStream,
    TryStreamExt,
};
use kube::api::{Api, DynamicObject, ListParams, Resource};
use serde::de::DeserializeOwned;
use snafu::{futures::TryStreamExt as SnafuTryStreamExt, Backtrace, ResultExt, Snafu};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
    time::Duration,
};
use stream::BoxStream;
use tokio::{runtime::Handle, time::Instant};
use tracing::{info_span, Instrument};

mod future_hash_map;
mod runner;

#[derive(Snafu, Debug)]
pub enum Error<ReconcilerErr: std::error::Error + 'static, QueueErr: std::error::Error + 'static> {
    ObjectNotFound {
        obj_ref: ObjectRef<DynamicObject>,
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

/// Results of the reconciliation attempt
#[derive(Debug, Clone)]
pub struct ReconcilerAction {
    /// Whether (and when) to next trigger the reconciliation if no external watch triggers hit
    ///
    /// For example, use this to query external systems for updates, expire time-limited resources, or
    /// (in your `error_policy`) retry after errors.
    pub requeue_after: Option<Duration>,
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
            .filter_map(move |owner| ObjectRef::from_owner_ref(ns.as_deref(), &owner, owner_type.clone()))
            .map(move |owner_ref| ReconcileRequest {
                obj_ref: owner_ref,
                reason: ReconcileReason::RelatedObjectUpdated {
                    obj_ref: child_ref.clone(),
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
    RelatedObjectUpdated { obj_ref: ObjectRef<DynamicObject> },
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
    mut reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
    mut error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> ReconcilerAction,
    context: Context<T>,
    store: Store<K>,
    queue: QueueStream,
) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error, QueueStream::Error>>>
where
    K: Clone + Resource + 'static,
    K::DynamicType: Debug + Eq + Hash + Clone + Unpin,
    ReconcilerFut: TryFuture<Ok = ReconcilerAction> + Unpin,
    ReconcilerFut::Error: std::error::Error + 'static,
    QueueStream: TryStream,
    QueueStream::Ok: Into<ReconcileRequest<K>>,
    QueueStream::Error: std::error::Error + 'static,
{
    let err_context = context.clone();
    let (scheduler_tx, scheduler_rx) = channel::mpsc::channel::<ScheduleRequest<ReconcileRequest<K>>>(100);
    // Create a stream of ObjectRefs that need to be reconciled
    trystream_try_via(
        // input: stream combining scheduled tasks and user specified inputs event
        Box::pin(stream::select(
            // 1. inputs from users queue stream
            queue.context(QueueError).map_ok(|request| ScheduleRequest {
                message: request.into(),
                run_at: Instant::now() + Duration::from_millis(1),
            }),
            // 2. requests sent to scheduler_tx
            scheduler_rx.map(Ok),
        )),
        // all the Oks from the select gets passed through the scheduler stream, and are then executed
        move |s| {
            Runner::new(scheduler(s), move |request| {
                let request = request.clone();
                match store.get(&request.obj_ref) {
                    Some(obj) => reconciler(obj, context.clone())
                        .into_future()
                        .instrument(info_span!("reconciling object", "object.ref" = %request.obj_ref, object.reason = %request.reason))
                        // Reconciler errors are OK from the applier's PoV, we need to apply the error policy
                        // to them separately
                        .map(|res| Ok((request.obj_ref, res)))
                        .left_future(),
                    None => future::err(
                        ObjectNotFound {
                            obj_ref: request.obj_ref.erase(),
                        }
                        .build(),
                    )
                    .right_future(),
                }
            })
            .context(SchedulerDequeueFailed)
            .map(|res| res.and_then(|x| x))
        },
    )
    // finally, for each completed reconcile call:
    .and_then(move |(obj_ref, reconciler_result)| {
        let (ReconcilerAction { requeue_after }, requeue_reason) = match &reconciler_result {
            Ok(action) =>
                // do what user told us
                (action.clone(), ReconcileReason::ReconcilerRequestedRetry),
            Err(err) =>
                // reconciler fn call failed
                (error_policy(err, err_context.clone()), ReconcileReason::ErrorPolicyRequestedRetry),
        };
        let mut scheduler_tx = scheduler_tx.clone();
        async move {
            // Transmit the requeue request to the scheduler (picked up again at top)
            if let Some(delay) = requeue_after {
                scheduler_tx
                    .send(ScheduleRequest {
                        message: ReconcileRequest {obj_ref: obj_ref.clone(), reason: requeue_reason},
                        run_at: Instant::now() + delay,
                    })
                    .await
                    .expect("Message could not be sent to scheduler_rx");
            }
            reconciler_result
                .map(|action| (obj_ref, action))
                .context(ReconcilerFailed)
        }
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
/// use schemars::JsonSchema;
///
/// use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
/// #[derive(Debug, Snafu)]
/// enum Error {}
/// /// A custom resource
/// #[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
/// #[kube(group = "nullable.se", version = "v1", kind = "ConfigMapGenerator", namespaced)]
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
/// /// an error handler that will be called when the reconciler fails
/// fn error_policy(_error: &Error, _ctx: Context<()>) -> ReconcilerAction {
///     ReconcilerAction {
///         requeue_after: Some(Duration::from_secs(60)),
///     }
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
    // TODO: get an arbitrary std::error::Error in here?
    trigger_selector: stream::SelectAll<BoxStream<'static, Result<ReconcileRequest<K>, watcher::Error>>>,
    dyntype: K::DynamicType,
    reader: Store<K>,
}

impl<K> Controller<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Eq + Hash + Clone + Default,
{
    /// Create a Controller on a type `K`
    ///
    /// Configure `ListParams` and `Api` so you only get reconcile events
    /// for the correct `Api` scope (cluster/all/namespaced), or `ListParams` subset
    #[must_use]
    pub fn new(owned_api: Api<K>, lp: ListParams) -> Self {
        Self::new_with(owned_api, lp, Default::default())
    }
}

impl<K> Controller<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Eq + Hash + Clone,
{
    /// Create a Controller on a type `K`
    ///
    /// Configure `ListParams` and `Api` so you only get reconcile events
    /// for the correct `Api` scope (cluster/all/namespaced), or `ListParams` subset
    ///
    /// Unlike `new`, this function accepts `K::DynamicType` so it can be used with dynamic
    /// resources.
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
            dyntype,
            reader,
        }
    }

    /// Retrieve a copy of the reader before starting the controller
    pub fn store(&self) -> Store<K> {
        self.reader.clone()
    }

    /// Indicate child objets `K` owns and be notified when they change
    ///
    /// This type `Child` must have [`OwnerReference`] set to point back to `K`.
    /// You can customize the parameters used by the underlying `watcher` if
    /// only a subset of `Child` entries are required.
    /// The `api` must have the correct scope (cluster/all namespaces, or namespaced)
    ///
    /// [`OwnerReference`]: https://docs.rs/k8s-openapi/0.10.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.OwnerReference.html
    pub fn owns<Child: Clone + Resource + DeserializeOwned + Debug + Send + 'static>(
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

    /// Indicate an object to watch with a custom mapper
    ///
    /// This mapper should return something like `Option<ObjectRef<K>>`
    pub fn watches<
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
                        obj_ref: watched_obj_ref.clone(),
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
    /// ```rust
    /// # async fn foo() {
    /// use futures::stream::StreamExt;
    /// use k8s_openapi::api::core::v1::ConfigMap;
    /// use kube::{api::ListParams, Api, Client, ResourceExt};
    /// use kube_runtime::controller::{Context, Controller, ReconcilerAction};
    /// use std::convert::Infallible;
    /// use tokio::io::{stdin, AsyncBufReadExt, BufReader};
    /// use tokio_stream::wrappers::LinesStream;
    /// Controller::new(
    ///     Api::<ConfigMap>::all(Client::try_default().await.unwrap()),
    ///     ListParams::default(),
    /// )
    /// .reconcile_all_on(LinesStream::new(BufReader::new(stdin()).lines()).map(|_| ()))
    /// .run(
    ///     |o, _| async move {
    ///         println!("Reconciling {}", o.name());
    ///         Ok(ReconcilerAction { requeue_after: None })
    ///     },
    ///     |err: &Infallible, _| Err(err).unwrap(),
    ///     Context::new(()),
    /// );
    /// # }
    /// ```
    pub fn reconcile_all_on(mut self, trigger: impl Stream<Item = ()> + Send + Sync + 'static) -> Self {
        let store = self.store();
        let dyntype = self.dyntype.clone();
        self.trigger_selector.push(
            trigger
                .flat_map(move |()| {
                    let dyntype = dyntype.clone();
                    stream::iter(store.state().into_iter().map(move |obj| {
                        Ok(ReconcileRequest {
                            obj_ref: ObjectRef::from_obj_with(&obj, dyntype.clone()),
                            reason: ReconcileReason::BulkReconcile,
                        })
                    }))
                })
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
        mut reconciler: impl FnMut(K, Context<T>) -> ReconcilerFut,
        error_policy: impl FnMut(&ReconcilerFut::Error, Context<T>) -> ReconcilerAction,
        context: Context<T>,
    ) -> impl Stream<Item = Result<(ObjectRef<K>, ReconcilerAction), Error<ReconcilerFut::Error, watcher::Error>>>
    where
        K::DynamicType: Debug + Unpin,
        ReconcilerFut: TryFuture<Ok = ReconcilerAction> + Send + 'static,
        ReconcilerFut::Error: std::error::Error + Send + 'static,
    {
        applier(
            move |obj, ctx| {
                CancelableJoinHandle::spawn(reconciler(obj, ctx).into_future(), &Handle::current())
            },
            error_policy,
            context,
            self.reader,
            self.trigger_selector,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, ReconcilerAction};
    use crate::Controller;
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::Api;

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
                |_, _| async { Ok(mock_type::<ReconcilerAction>()) },
                |_: &std::io::Error, _| mock_type::<ReconcilerAction>(),
                Context::new(()),
            ),
        );
    }
}
