use crate::{
    reflector::{
        reflector,
        store::{Store, Writer},
    },
    utils::{stream_subscribe, StreamSubscribe, WatchStreamExt},
    watcher::{self, watcher, Event},
};
use futures::{stream, Stream, StreamExt};
use k8s_openapi::NamespaceResourceScope;
use kube_client::{api::ListParams, Api, Client, Resource};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};
use stream::BoxStream;

pub struct SharedStore<K, W = WatcherFactory<K>>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
    K::DynamicType: Hash + Eq,
    W: CreateWatcher<K>,
{
    watcher_provider: W,
    reflectors: HashMap<ScopedListParams, (Store<K>, BoxStreamSubscribe<K>)>,
}

type ScopedListParams = (Option<String>, ListParams);
type BoxStreamSubscribe<K> = StreamSubscribe<BoxStream<'static, watcher::Result<Event<K>>>>;

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
    pub fn run(self) -> impl Stream<Item = Arc<watcher::Result<Event<K>>>> {
        stream::select_all(self.reflectors.into_iter().map(|(_, (_, reflector))| reflector))
    }

    pub fn namespaced(&mut self, namespace: &str, list_params: ListParams) -> Store<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        self.reflectors
            .entry((Some(namespace.to_string()), list_params.clone()))
            .or_insert_with(|| setup_reflector(self.watcher_provider.namespaced(namespace, list_params)))
            .0
            .clone()
    }

    pub fn all(&mut self, list_params: ListParams) -> Store<K> {
        self.reflectors
            .entry((None, list_params.clone()))
            .or_insert_with(|| setup_reflector(self.watcher_provider.all(list_params)))
            .0
            .clone()
    }

    pub fn subscribe_namespaced(
        &mut self,
        namespace: &str,
        list_params: ListParams,
    ) -> impl Stream<Item = Result<Arc<watcher::Result<Event<K>>>, stream_subscribe::Error>>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        self.reflectors
            .entry((Some(namespace.to_string()), list_params.clone()))
            .or_insert_with(|| setup_reflector(self.watcher_provider.namespaced(namespace, list_params)))
            .1
            .subscribe()
    }

    pub fn subscribe_all(
        &mut self,
        list_params: ListParams,
    ) -> impl Stream<Item = Result<Arc<watcher::Result<Event<K>>>, stream_subscribe::Error>>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        self.reflectors
            .entry((None, list_params.clone()))
            .or_insert_with(|| setup_reflector(self.watcher_provider.all(list_params)))
            .1
            .subscribe()
    }
}

fn setup_reflector<K>(
    watcher: BoxStream<'static, watcher::Result<Event<K>>>,
) -> (Store<K>, BoxStreamSubscribe<K>)
where
    K: Resource + Clone + Send + Sync,
    K::DynamicType: Default + Eq + Hash + Clone,
{
    let store_writer = Writer::default();
    let store_reader = store_writer.as_reader();
    let reflector = reflector(store_writer, watcher).boxed().stream_subscribe();

    (store_reader, reflector)
}

pub trait CreateWatcher<K> {
    fn all(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>>;

    fn namespaced(
        &self,
        namespace: &str,
        list_params: ListParams,
    ) -> BoxStream<'static, watcher::Result<Event<K>>>
    where
        K: Resource<Scope = NamespaceResourceScope>;
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
    fn all(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>> {
        watcher(Api::all(self.client.clone()), list_params).boxed()
    }

    fn namespaced(
        &self,
        namespace: &str,
        list_params: ListParams,
    ) -> BoxStream<'static, watcher::Result<Event<K>>>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        watcher(Api::namespaced(self.client.clone(), namespace), list_params).boxed()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::expect_used)]
mod test {
    use super::*;
    use futures::stream;
    use k8s_openapi::{
        api::core::v1::{ConfigMap, Pod},
        apimachinery::pkg::apis::meta::v1::ObjectMeta,
    };
    use maplit::hashmap;
    use std::{
        collections::VecDeque,
        fmt::{Display, Formatter},
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[tokio::test]
    async fn returns_stores_that_updates_on_events() {
        let lp = ListParams::default();
        let expected_state = vec![test_pod(1)];
        let mut pod_ss = TestSharedStore::<Pod>::new(
            hashmap!((None, lp.clone().into()) => vec![Event::Restarted(expected_state.clone())]),
        );
        let store = pod_ss.all(lp);

        pod_ss.spawn().await;

        assert_eq!(store.cloned_state(), expected_state);
    }

    #[tokio::test]
    async fn returns_the_same_store_for_the_same_list_params() {
        let lp = ListParams::default().labels("foo=bar");
        let expected_state = vec![test_pod(1)];
        let mut pod_ss = TestSharedStore::<Pod>::new(
            hashmap!((None, lp.clone().into()) => vec![Event::Restarted(expected_state.clone())]),
        );

        let store1 = pod_ss.all(lp.clone());
        let store2 = pod_ss.all(lp);

        pod_ss.spawn().await;

        assert_eq!(store1.cloned_state(), expected_state);
        assert_eq!(store2.cloned_state(), expected_state);
    }

    #[tokio::test]
    async fn returns_a_different_store_for_different_list_params() {
        let lp1 = ListParams::default().labels("foo=bar");
        let lp2 = ListParams::default().labels("foo=baz");
        let expected_state1 = vec![test_pod(1)];
        let expected_state2 = vec![test_pod(2)];
        let mut ss = TestSharedStore::<Pod>::new(hashmap!(
            (None, lp1.clone().into()) => vec![Event::Restarted(expected_state1.clone())],
            (None, lp2.clone().into()) => vec![Event::Restarted(expected_state2.clone())],
        ));

        let store1 = ss.all(lp1);
        let store2 = ss.all(lp2);

        ss.spawn().await;

        assert_eq!(store1.cloned_state(), expected_state1, "Store 1");
        assert_eq!(store2.cloned_state(), expected_state2, "Store 2");
    }

    #[tokio::test]
    async fn returns_different_stores_for_same_list_params_with_different_scope() {
        let lp = ListParams::default().labels("foo=bar");
        let ns = "ns1";
        let expected_state1 = vec![test_pod(1)];
        let expected_state2 = vec![test_pod(2)];
        let mut ss = TestSharedStore::<Pod>::new(hashmap!(
            (None, lp.clone().into()) => vec![Event::Restarted(expected_state1.clone())],
            (Some(ns.to_string()), lp.clone().into()) => vec![Event::Restarted(expected_state2.clone())],
        ));

        let cluster_store1 = ss.all(lp.clone());
        let ns_store1 = ss.namespaced(ns, lp.clone());
        let cluster_store2 = ss.all(lp);

        ss.spawn().await;

        assert_eq!(cluster_store1.cloned_state(), expected_state1, "ClusterStore 1");
        assert_eq!(ns_store1.cloned_state(), expected_state2, "NS Store 1");
        assert_eq!(cluster_store2.cloned_state(), expected_state1, "Cluster Store 2");
    }

    // TODO - Tests for the subscriptions

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

    type TestSharedStore<K> = SharedStore<K, TestWatcherFactory<K>>;

    impl<K> SharedStore<K, TestWatcherFactory<K>>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
        K::DynamicType: Clone + Debug + Default + Eq + Hash + Unpin,
    {
        fn new(events: HashMap<(Option<String>, ListParams), Vec<Event<K>>>) -> Self {
            Self {
                watcher_provider: TestWatcherFactory {
                    events: Mutex::new(events.into_iter().map(|(k, v)| (k, v.into())).collect()),
                },
                reflectors: HashMap::new(),
            }
        }

        async fn spawn(self) {
            tokio::spawn(async move {
                self.run().for_each(|_| async {}).await;
            });

            tokio::task::yield_now().await;
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

    struct TestWatcherFactory<K> {
        events: Mutex<HashMap<(Option<String>, ListParams), VecDeque<Event<K>>>>,
    }

    impl<K> CreateWatcher<K> for TestWatcherFactory<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
        K::DynamicType: Hash + Eq,
    {
        fn all(&self, list_params: ListParams) -> BoxStream<'static, watcher::Result<Event<K>>> {
            self.watcher(None, list_params).boxed()
        }

        fn namespaced(
            &self,
            namespace: &str,
            list_params: ListParams,
        ) -> BoxStream<'static, watcher::Result<Event<K>>>
        where
            K: Resource<Scope = NamespaceResourceScope>,
        {
            self.watcher(Some(namespace.to_string()), list_params).boxed()
        }
    }

    impl<K> TestWatcherFactory<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
        K::DynamicType: Hash + Eq,
    {
        fn watcher(
            &self,
            scope: Option<String>,
            list_params: ListParams,
        ) -> impl Stream<Item = watcher::Result<Event<K>>> + Send {
            let events = self
                .events
                .lock()
                .unwrap()
                .remove(&(scope, list_params.into()))
                .expect("There can be only one stream per ListParams");

            stream::unfold(events, |mut events| async move {
                match events.pop_front() {
                    Some(event) => Some((Ok(event), events)),
                    // if there's nothing left we block to simulate waiting for a change
                    None => futures::future::pending().await,
                }
            })
        }
    }

    enum TestError {
        TestError,
    }

    impl Debug for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            unimplemented!()
        }
    }

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            unimplemented!()
        }
    }

    impl std::error::Error for TestError {}

    trait ClonedState<K> {
        fn cloned_state(&self) -> Vec<K>;
    }

    impl<K> ClonedState<K> for Store<K>
    where
        K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
        K::DynamicType: Clone + Debug + Default + Eq + Hash + Unpin,
    {
        fn cloned_state(&self) -> Vec<K> {
            self.state().into_iter().map(|k| (*k).clone()).collect::<Vec<_>>()
        }
    }
}
