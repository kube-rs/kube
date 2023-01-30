mod ready_token;

use crate::controller::{trigger_self, Action};
use crate::shared_store::ready_token::ReadyToken;
use crate::utils::StreamSubscribable;
use crate::watcher::Event;
use crate::{
    applier,
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef,
    },
    utils::{CancelableJoinHandle, StreamBackoff, WatchStreamExt},
    watcher::{self, watcher},
};
use futures::{stream, Stream, StreamExt, TryFuture, TryFutureExt, TryStreamExt};
use kube_client::api::ListParams;
use kube_client::{Api, Resource};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::{fmt::Debug, hash::Hash, sync::Arc};
use stream::BoxStream;
use tokio::runtime::Handle;
use tracing::Instrument;

// TODO - Not sure this is the right name?
pub struct SharedStore<K, W = WatcherProvider<K>>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
    K::DynamicType: Hash + Eq,
    W: CreateWatcher<K>,
{
    watcher_provider: W,
    reflectors: HashMap<ListParams, (Store<K>, StreamSubscribable<K>)>,
    controllers: Vec<BoxStream<'static, ControllerResult<K>>>,
    ready_token: ReadyToken,
}

// TODO - not sure about the error type here, we might not need to box it?
type ControllerResult<K> = Result<(ObjectRef<K>, Action), Box<dyn std::error::Error>>;

impl<K> SharedStore<K>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
    K::DynamicType: Default + Eq + Hash + Clone,
{
    pub fn new(api: Api<K>, ready_token: &ReadyToken) -> Self {
        Self {
            watcher_provider: WatcherProvider::new(api),
            reflectors: HashMap::new(),
            controllers: Vec::new(),
            ready_token: ready_token.clone(),
        }
    }
}

impl<K, W> SharedStore<K, W>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Default + Eq + Hash + Clone,
    W: CreateWatcher<K> + 'static,
{
    pub fn run(self) -> impl Stream<Item = ProviderResult<K>> {
        let reflectors = stream::select_all(self.reflectors.into_iter().map(|(_, (_, reflector))| reflector));

        let have_controllers = !self.controllers.is_empty();
        let controllers = stream::select_all(self.controllers);

        // todo - make sure that if ANY stream dies we die

        stream::unfold(
            (reflectors, controllers, self.ready_token),
            move |(mut reflectors, mut controllers, ready_token)| async move {
                tokio::select!(
                    result = reflectors.next() => {
                        result.map(|r| (ProviderResult::Reflector(r), (reflectors, controllers, ready_token)))
                    },
                    result = controllers.next(), if have_controllers && ready_token.is_ready() => {
                        result.map(|r| (ProviderResult::Controller(r), (reflectors, controllers, ready_token)))
                    }
                )
            },
        )
    }

    // todo - we still need to be able to setup "watches" and "owns"
    pub fn controller<ReconcilerFut, Ctx>(
        &mut self,
        list_params: ListParams,
        mut reconciler: impl FnMut(Arc<K>, Arc<Ctx>) -> ReconcilerFut + Send + 'static,
        error_policy: impl Fn(Arc<K>, &ReconcilerFut::Error, Arc<Ctx>) -> Action + Send + Sync + 'static,
        context: Arc<Ctx>,
    ) where
        K::DynamicType: Debug + Unpin,
        ReconcilerFut: TryFuture<Ok = Action> + Send + 'static,
        ReconcilerFut::Error: std::error::Error + Send + 'static,
        Ctx: Send + Sync + 'static,
    {
        let dyntype = K::DynamicType::default();

        let (store, event_stream) = self.reflector(list_params.clone().into());
        let self_watcher = trigger_self(event_stream.map(Ok).applied_objects(), dyntype).boxed();

        let mut trigger_selector = stream::SelectAll::new();
        trigger_selector.push(self_watcher);

        let trigger_backoff = Box::new(watcher::default_backoff());

        let stream = applier(
            move |obj, ctx| {
                CancelableJoinHandle::spawn(
                    reconciler(obj, ctx).into_future().in_current_span(),
                    &Handle::current(),
                )
            },
            error_policy,
            context,
            store,
            StreamBackoff::new(trigger_selector, trigger_backoff),
        )
        .map(|result| result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>));

        self.controllers.push(stream.boxed());
    }

    pub fn store(&mut self, list_params: ListParams) -> Store<K> {
        self.reflector(list_params).0
    }

    fn reflector(&mut self, list_params: ListParams) -> (Store<K>, impl Stream<Item = Event<K>>) {
        if let Some((store, prism)) = self.reflectors.get(&list_params) {
            return (store.clone(), prism.subscribe_ok());
        }

        let watcher = self.watcher_provider.watcher(list_params.clone());
        let store_writer = Writer::default();
        let store_reader = store_writer.as_reader();

        let ready_state = self.ready_token.child();

        let reflector = reflector(store_writer, watcher).inspect_ok(move |_| ready_state.ready());

        let subscribable_reflector = reflector.subscribable();
        let event_stream = subscribable_reflector.subscribe_ok();

        self.reflectors.insert(
            list_params.clone(),
            (store_reader.clone(), subscribable_reflector),
        );

        (store_reader, event_stream)
    }
}

pub trait CreateWatcher<K>
where
    K: Resource + Clone + DeserializeOwned + Debug + Send + 'static,
{
    fn watcher(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>>;
}

pub struct WatcherProvider<K>
where
    K: Resource + Clone + DeserializeOwned + Debug + Send + 'static,
{
    api: Api<K>,
}

impl<K> WatcherProvider<K>
where
    K: Resource + Clone + DeserializeOwned + Debug + Send + 'static,
{
    fn new(api: Api<K>) -> Self {
        Self { api }
    }
}

impl<K> CreateWatcher<K> for WatcherProvider<K>
where
    K: Resource + Clone + DeserializeOwned + Debug + Send + 'static,
{
    fn watcher(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>> {
        watcher(self.api.clone(), list_params).boxed()
    }
}

pub enum ProviderResult<K: Resource> {
    Reflector(watcher::Result<Event<K>>),
    Controller(Result<(ObjectRef<K>, Action), Box<dyn std::error::Error>>),
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::expect_used)]
mod test {
    use super::*;
    use futures::stream;
    use k8s_openapi::api::core::v1::{ConfigMap, Pod};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use maplit::hashmap;
    use std::collections::VecDeque;
    use std::fmt::{Display, Formatter};
    use std::sync::Mutex;
    use std::time::Duration;

    mod store {
        use super::*;

        #[tokio::test]
        async fn it_returns_a_store() {
            let lp = ListParams::default();
            let mut provider =
                TestProvider::<Pod>::new(hashmap!(lp.clone().into() => vec![]), &ReadyToken::new());
            let store = provider.store(lp);
            assert_eq!(store.state().len(), 0);
        }

        #[tokio::test]
        async fn it_returns_stores_that_updates_on_events() {
            let lp = ListParams::default();
            let expected_state = vec![test_pod(1)];
            let mut kp = TestProvider::<Pod>::new(
                hashmap!(lp.clone().into() => vec![Event::Restarted(expected_state.clone())]),
                &ReadyToken::new(),
            );
            let store = kp.store(lp);

            kp.spawn().await;

            assert_eq!(store.cloned_state(), expected_state);
        }

        #[tokio::test]
        async fn it_returns_the_same_store_for_the_same_list_params() {
            let lp = ListParams::default().labels("foo=bar");
            let expected_state = vec![test_pod(1)];
            let mut provider = TestProvider::<Pod>::new(
                hashmap!(lp.clone().into() => vec![Event::Restarted(expected_state.clone())]),
                &ReadyToken::new(),
            );

            let store1 = provider.store(lp.clone());
            let store2 = provider.store(lp);

            provider.spawn().await;

            assert_eq!(store1.cloned_state(), expected_state);
            assert_eq!(store2.cloned_state(), expected_state);
        }

        #[tokio::test]
        async fn it_returns_a_different_store_for_different_list_params() {
            let lp1 = ListParams::default().labels("foo=bar");
            let lp2 = ListParams::default().labels("foo=baz");
            let expected_state1 = vec![test_pod(1)];
            let expected_state2 = vec![test_pod(2)];
            let mut kp = TestProvider::<Pod>::new(
                hashmap!(
                    lp1.clone().into() => vec![Event::Restarted(expected_state1.clone())],
                    lp2.clone().into() => vec![Event::Restarted(expected_state2.clone())],
                ),
                &ReadyToken::new(),
            );

            let store1 = kp.store(lp1);
            let store2 = kp.store(lp2);

            kp.spawn().await;

            assert_eq!(store1.cloned_state(), expected_state1, "Store 1");
            assert_eq!(store2.cloned_state(), expected_state2, "Store 2");
        }
    }

    mod controller {
        use super::*;
        use futures::stream::select_all;
        use k8s_openapi::api::core::v1::ConfigMap;

        #[tokio::test]
        async fn it_creates_a_controller() {
            let lp = ListParams::default().labels("foo=baz");
            let expected_state = vec![test_pod(2)];
            let mut provider = TestProvider::<Pod>::new(
                hashmap!(
                    lp.clone().into() => vec![Event::Restarted(vec![expected_state[0].clone()])],
                ),
                &ReadyToken::new(),
            );

            let context = Arc::new(Mutex::new(vec![]));
            provider.controller(
                lp.clone(),
                |pod, ctx| async move {
                    ctx.lock().unwrap().push(pod.clone());
                    Ok::<_, TestError>(Action::await_change())
                },
                |_, _, _| Action::await_change(),
                context.clone(),
            );

            let store = provider.store(lp.clone());

            provider.spawn().await;

            assert_eq!(store.cloned_state(), expected_state, "Store");
            assert_eq!(context.cloned_state(), expected_state, "Context");
        }

        #[tokio::test]
        async fn it_doesnt_run_the_controller_until_the_store_has_received_a_first_event() {
            let lp = ListParams::default().labels("foo=baz");
            let mut kp = TestProvider::<Pod>::new(
                hashmap!(
                    lp.clone().into() => vec![],

                ),
                &ReadyToken::new(),
            );

            let context = kp.test_controller(lp.clone());

            kp.spawn().await;

            assert!(!context.reconciled());
        }

        #[tokio::test]
        async fn many_controllers_wait_until_every_store_is_ready() {
            let lp = ListParams::default().labels("foo=baz");
            let ready_token = ReadyToken::new();
            let mut kp1 = TestProvider::<Pod>::new(
                hashmap!(
                    lp.clone().into() => vec![],
                ),
                &ready_token,
            );
            let mut kp2 = TestProvider::<ConfigMap>::new(
                hashmap!(
                    lp.clone().into() => vec![],
                ),
                &ready_token,
            );

            let context1 = kp1.test_controller(lp.clone());
            let context2 = kp2.test_controller(lp.clone());

            kp1.spawn().await;
            kp2.spawn().await;

            assert!(!context1.reconciled());
            assert!(!context2.reconciled());
        }

        #[tokio::test]
        async fn it_doesnt_run_a_controller_if_a_store_from_another_provider_isnt_ready() {
            let lp = ListParams::default().labels("foo=baz");
            let ready_token = ReadyToken::new();
            let mut kp1 = TestProvider::<Pod>::new(
                hashmap!(
                    lp.clone().into() => vec![Event::Restarted(vec![test_pod(1).clone()])],
                ),
                &ready_token,
            );
            let mut kp2 = TestProvider::<ConfigMap>::new(
                hashmap!(
                    lp.clone().into() => vec![],
                ),
                &ready_token,
            );

            let context = kp1.test_controller(lp.clone());
            let _store = kp2.store(lp.clone());

            kp1.spawn().await;
            kp2.spawn().await;

            assert!(!ready_token.is_ready(), "Ready token");
            assert!(!context.reconciled(), "Context");
        }

        #[tokio::test]
        async fn many_controllers_run_after_all_stores_are_ready() {
            let lp = ListParams::default().labels("foo=baz");
            let ready_token = ReadyToken::new();
            let mut provider1 = TestProvider::<Pod>::new(
                hashmap!(
                    lp.clone().into() => vec![Event::Restarted(vec![test_pod(1).clone()])],
                ),
                &ready_token,
            );
            let mut provider2 = TestProvider::<ConfigMap>::new(
                hashmap!(
                    lp.clone().into() => vec![Event::Restarted(vec![test_cm(1)])],
                ),
                &ready_token,
            );

            let context1 = provider1.test_controller(lp.clone());
            let context2 = provider2.test_controller(lp.clone());

            tokio::spawn(async move {
                select_all(vec![
                    provider1.run().map(|_| ()).boxed(),
                    provider2.run().map(|_| ()).boxed(),
                ])
                .for_each(|_| async {})
                .await;
            });

            tokio::time::sleep(Duration::from_millis(10)).await;

            assert!(ready_token.is_ready(), "ReadyToken");
            assert!(context1.reconciled(), "Context 1");
            assert!(context2.reconciled(), "Context 2");
        }
    }

    fn test_pod(postfix: usize) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(format!("test-pod-{}", postfix)),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn test_cm(postfix: usize) -> ConfigMap {
        ConfigMap {
            metadata: ObjectMeta {
                name: Some(format!("test-pod-{}", postfix)),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    type TestProvider<K> = SharedStore<K, TestWatcherProvider<K>>;

    impl<K> SharedStore<K, TestWatcherProvider<K>>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
        K::DynamicType: Clone + Debug + Default + Eq + Hash + Unpin,
    {
        fn new(events: HashMap<ListParams, Vec<Event<K>>>, ready_token: &ReadyToken) -> Self {
            Self {
                watcher_provider: TestWatcherProvider {
                    events: Mutex::new(events.into_iter().map(|(k, v)| (k, v.into())).collect()),
                },
                reflectors: HashMap::new(),
                controllers: Vec::new(),
                ready_token: ready_token.clone(),
            }
        }

        fn test_controller(&mut self, list_params: ListParams) -> TestContext {
            let context = TestContext::new();
            self.controller(
                list_params,
                |_, ctx| async move {
                    *(ctx.reconciled.lock().unwrap()) = true;
                    Ok::<_, TestError>(Action::await_change())
                },
                |_, _, _| Action::await_change(),
                Arc::new(context.clone()),
            );
            context
        }

        async fn spawn(self) {
            tokio::spawn(async move {
                self.run().for_each(|_| async {}).await;
            });

            // We have to sleep here to give the scheduling stuff inside KubeRS chance to
            // schedule the events that are fed into it
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[derive(Clone, Debug)]
    struct TestContext {
        reconciled: Arc<Mutex<bool>>,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                reconciled: Arc::new(Mutex::new(false)),
            }
        }

        fn reconciled(&self) -> bool {
            *self.reconciled.lock().unwrap()
        }
    }

    struct TestWatcherProvider<K> {
        events: Mutex<HashMap<ListParams, VecDeque<Event<K>>>>,
    }

    impl<K> CreateWatcher<K> for TestWatcherProvider<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
        K::DynamicType: Hash + Eq,
    {
        fn watcher(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>> {
            let events = self
                .events
                .lock()
                .unwrap()
                .remove(&list_params.into())
                .expect("There can be only one stream per ListParams");

            stream::unfold(events, |mut events| async move {
                match events.pop_front() {
                    Some(event) => Some((Ok(event), events)),
                    // if there's nothing left we block to simulate waiting for a change
                    None => futures::future::pending().await,
                }
            })
            .boxed()
        }
    }

    trait ClonedState<K> {
        fn cloned_state(&self) -> Vec<K>;
    }

    impl<K> ClonedState<K> for Store<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
        K::DynamicType: Clone + Default + Eq + Hash,
    {
        fn cloned_state(&self) -> Vec<K> {
            self.state().into_iter().map(|k| (*k).clone()).collect::<Vec<_>>()
        }
    }

    impl<K: Clone> ClonedState<K> for Arc<Mutex<Vec<Arc<K>>>> {
        fn cloned_state(&self) -> Vec<K> {
            self.lock()
                .unwrap()
                .iter()
                .map(|k| (**k).clone())
                .collect::<Vec<_>>()
        }
    }

    enum TestError {
        TestError,
    }

    impl Debug for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl std::error::Error for TestError {}
}
