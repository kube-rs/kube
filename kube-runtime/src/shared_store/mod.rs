mod ready_token;
mod safe_store;

use crate::controller::Action;
use crate::shared_store::safe_store::SafeStore;
use crate::utils::StreamSubscribable;
use crate::watcher::Event;
use crate::{
    reflector::{
        reflector,
        store::{Store, Writer},
        ObjectRef,
    },
    utils::{CancelableJoinHandle, StreamBackoff, WatchStreamExt},
    watcher::{self, watcher},
};
use futures::{stream, Stream, StreamExt, TryFuture, TryFutureExt, TryStream, TryStreamExt};
use k8s_openapi::NamespaceResourceScope;
use kube_client::api::ListParams;
use kube_client::{Api, Client, Resource};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use stream::BoxStream;

pub struct SharedStore<K, W = WatcherFactory<K>>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
    K::DynamicType: Hash + Eq,
    W: CreateWatcher<K>,
{
    watcher_provider: W,
    reflectors: HashMap<(Option<String>, ListParams), (SafeStore<K>, SubscribableBoxStream<K>)>,
}

type SubscribableBoxStream<K> = StreamSubscribable<BoxStream<'static, watcher::Result<Event<K>>>>;

impl<K> SharedStore<K>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
    K::DynamicType: Default + Eq + Hash + Clone,
{
    pub fn new(client: Client) -> Self {
        Self {
            watcher_provider: WatcherFactory::new(client),
            reflectors: HashMap::new(),
        }
    }
}

impl<K, W> SharedStore<K, W>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Default + Eq + Hash + Clone,
    W: CreateWatcher<K> + 'static,
{
    pub fn run(self) -> impl Stream<Item = watcher::Result<Event<K>>> {
        stream::select_all(self.reflectors.into_iter().map(|(_, (_, reflector))| reflector))
    }

    pub fn namespaced(&mut self, namespace: &str, list_params: ListParams) -> SafeStore<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        if let Some((store, _)) = self
            .reflectors
            .get(&(Some(namespace.to_string()), list_params.clone()))
        {
            return store.clone();
        }

        let watcher = self.watcher_provider.namespaced(namespace, list_params.clone());

        self.reflector(watcher, Some(namespace.to_string()), list_params)
    }

    pub fn all(&mut self, list_params: ListParams) -> SafeStore<K> {
        if let Some((store, _)) = self.reflectors.get(&(None, list_params.clone())) {
            return store.clone();
        }

        let watcher = self.watcher_provider.all(list_params.clone());

        self.reflector(watcher, None, list_params)
    }

    pub fn subscribe_namespaced(
        &mut self,
        namespace: &str,
        list_params: ListParams,
    ) -> impl Stream<Item = Event<K>>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        if let Some((_, reflector)) = self
            .reflectors
            .get(&(Some(namespace.to_string()), list_params.clone()))
        {
            return reflector.subscribe_ok();
        }

        let watcher = self.watcher_provider.namespaced(namespace, list_params.clone());

        self.reflector(watcher, Some(namespace.to_string()), list_params.clone());

        // todo - We can safely unwrap here because we know we just created it ... but it's horrible, so we should fix it
        self.reflectors
            .get(&(Some(namespace.to_string()), list_params))
            .expect("reflector must exist")
            .1
            .subscribe_ok()
    }

    pub fn subscribe_all(&mut self, list_params: ListParams) -> impl Stream<Item = Event<K>>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        if let Some((_, reflector)) = self.reflectors.get(&(None, list_params.clone())) {
            return reflector.subscribe_ok();
        }

        let watcher = self.watcher_provider.all(list_params.clone());

        self.reflector(watcher, None, list_params.clone());

        // todo - We can safely unwrap here because we know we just created it ... but it's horrible, so we should fix it
        self.reflectors
            .get(&(None, list_params))
            .expect("reflector must exist")
            .1
            .subscribe_ok()
    }

    fn reflector(
        &mut self,
        pending_watcher: PendingWatcher<K>,
        scope: Option<String>,
        list_params: ListParams,
    ) -> SafeStore<K> {
        let store_writer = Writer::default();
        let store_reader = store_writer.as_reader();

        let safe_store = SafeStore::new(store_reader);

        // todo - maybe we want a "safe_reflector" ?
        let safe_store_clone = safe_store.clone();
        let reflector =
            reflector(store_writer, pending_watcher.run()).inspect_ok(move |_| safe_store_clone.make_ready());

        let subscribable_reflector = reflector.boxed().subscribable();
        let event_stream = subscribable_reflector.subscribe_ok();

        self.reflectors
            .insert((scope, list_params), (safe_store.clone(), subscribable_reflector));

        safe_store
    }
}

pub trait CreateWatcher<K> {
    fn all(&self, list_params: ListParams) -> PendingWatcher<K>;

    fn namespaced(&self, namespace: &str, list_params: ListParams) -> PendingWatcher<K>
    where
        K: Resource<Scope = NamespaceResourceScope>;
}

pub struct PendingWatcher<K>(BoxStream<'static, watcher::Result<Event<K>>>);

impl<K> PendingWatcher<K> {
    fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = watcher::Result<Event<K>>> + Send + 'static,
    {
        Self(stream.boxed())
    }

    fn run(self) -> impl Stream<Item = watcher::Result<Event<K>>> {
        self.0
    }
}

pub struct WatcherFactory<K> {
    client: Client,
    _phantom: std::marker::PhantomData<K>,
}

impl<K> WatcherFactory<K> {
    fn new(client: Client) -> Self {
        Self {
            client,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<K> CreateWatcher<K> for WatcherFactory<K>
where
    K: Resource + Clone + DeserializeOwned + Debug + Send + 'static,
    <K as Resource>::DynamicType: Default,
{
    fn all(&self, list_params: ListParams) -> PendingWatcher<K> {
        // is it worth catching the APIs within the provider?
        PendingWatcher::new(watcher(Api::all(self.client.clone()), list_params))
    }

    fn namespaced(&self, namespace: &str, list_params: ListParams) -> PendingWatcher<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        PendingWatcher::new(watcher(
            Api::namespaced(self.client.clone(), namespace),
            list_params,
        ))
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
    use async_trait::async_trait;
    use futures::stream;
    use k8s_openapi::api::core::v1::{ConfigMap, Pod};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use maplit::hashmap;
    use std::collections::VecDeque;
    use std::fmt::{Display, Formatter};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    mod store {
        use super::*;

        #[tokio::test]
        async fn it_returns_stores_that_updates_on_events() {
            let lp = ListParams::default();
            let expected_state = vec![test_pod(1)];
            let mut ss = TestProvider::<Pod>::new(
                hashmap!((None, lp.clone().into()) => vec![Event::Restarted(expected_state.clone())]),
            );
            let store = ss.all(lp);

            ss.spawn().await;

            assert_eq!(store.cloned_state().await, expected_state);
        }

        #[tokio::test]
        async fn it_returns_the_same_store_for_the_same_list_params() {
            let lp = ListParams::default().labels("foo=bar");
            let expected_state = vec![test_pod(1)];
            let mut ss = TestProvider::<Pod>::new(
                hashmap!((None, lp.clone().into()) => vec![Event::Restarted(expected_state.clone())]),
            );

            let store1 = ss.all(lp.clone());
            let store2 = ss.all(lp);

            ss.spawn().await;

            assert_eq!(store1.cloned_state().await, expected_state);
            assert_eq!(store2.cloned_state().await, expected_state);
        }

        #[tokio::test]
        async fn it_returns_a_different_store_for_different_list_params() {
            let lp1 = ListParams::default().labels("foo=bar");
            let lp2 = ListParams::default().labels("foo=baz");
            let expected_state1 = vec![test_pod(1)];
            let expected_state2 = vec![test_pod(2)];
            let mut ss = TestProvider::<Pod>::new(hashmap!(
                (None, lp1.clone().into()) => vec![Event::Restarted(expected_state1.clone())],
                (None, lp2.clone().into()) => vec![Event::Restarted(expected_state2.clone())],
            ));

            let store1 = ss.all(lp1);
            let store2 = ss.all(lp2);

            ss.spawn().await;

            assert_eq!(store1.cloned_state().await, expected_state1, "Store 1");
            assert_eq!(store2.cloned_state().await, expected_state2, "Store 2");
        }

        #[tokio::test]
        async fn it_returns_different_stores_by_scope() {
            let lp = ListParams::default().labels("foo=bar");
            let ns = "ns1";
            let expected_state1 = vec![test_pod(1)];
            let expected_state2 = vec![test_pod(2)];
            let mut ss = TestProvider::<Pod>::new(hashmap!(
                (None, lp.clone().into()) => vec![Event::Restarted(expected_state1.clone())],
                (Some(ns.to_string()), lp.clone().into()) => vec![Event::Restarted(expected_state2.clone())],
            ));

            let cluster_store1 = ss.all(lp.clone());
            let ns_store1 = ss.namespaced(ns, lp.clone());
            let cluster_store2 = ss.all(lp);

            ss.spawn().await;

            assert_eq!(
                cluster_store1.cloned_state().await,
                expected_state1,
                "ClusterStore 1"
            );
            assert_eq!(ns_store1.cloned_state().await, expected_state2, "NS Store 1");
            assert_eq!(
                cluster_store2.cloned_state().await,
                expected_state1,
                "Cluster Store 2"
            );
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
        fn new(events: HashMap<(Option<String>, ListParams), Vec<Event<K>>>) -> Self {
            Self {
                watcher_provider: TestWatcherProvider {
                    events: Mutex::new(events.into_iter().map(|(k, v)| (k, v.into())).collect()),
                },
                reflectors: HashMap::new(),
            }
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
        events: Mutex<HashMap<(Option<String>, ListParams), VecDeque<Event<K>>>>,
    }

    impl<K> CreateWatcher<K> for TestWatcherProvider<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
        K::DynamicType: Hash + Eq,
    {
        fn all(&self, list_params: ListParams) -> PendingWatcher<K> {
            self.watcher(None, list_params)
        }

        fn namespaced(&self, namespace: &str, list_params: ListParams) -> PendingWatcher<K>
        where
            K: Resource<Scope = NamespaceResourceScope>,
        {
            self.watcher(Some(namespace.to_string()), list_params)
        }
    }

    impl<K> TestWatcherProvider<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
        K::DynamicType: Hash + Eq,
    {
        fn watcher(&self, scope: Option<String>, list_params: ListParams) -> PendingWatcher<K> {
            let events = self
                .events
                .lock()
                .unwrap()
                .remove(&(scope, list_params.into()))
                .expect("There can be only one stream per ListParams");

            PendingWatcher::new(stream::unfold(events, |mut events| async move {
                match events.pop_front() {
                    Some(event) => Some((Ok(event), events)),
                    // if there's nothing left we block to simulate waiting for a change
                    None => futures::future::pending().await,
                }
            }))
        }
    }

    #[async_trait]
    trait ClonedState<K> {
        async fn cloned_state(&self) -> Vec<K>;
    }

    #[async_trait]
    impl<K> ClonedState<K> for SafeStore<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
        K::DynamicType: Clone + Default + Eq + Hash,
    {
        async fn cloned_state(&self) -> Vec<K> {
            self.state()
                .await
                .into_iter()
                .map(|k| (*k).clone())
                .collect::<Vec<_>>()
        }
    }

    #[async_trait]
    impl<K: Clone + Send + Sync> ClonedState<K> for Arc<Mutex<Vec<Arc<K>>>> {
        async fn cloned_state(&self) -> Vec<K> {
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
