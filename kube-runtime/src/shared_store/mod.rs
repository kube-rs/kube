mod ready_token;
mod safe_store;

use crate::controller::{trigger_self, Action};
use crate::shared_store::ready_token::ReadyToken;
use crate::shared_store::safe_store::SafeStore;
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
use futures::{stream, Stream, StreamExt, TryFuture, TryFutureExt, TryStream, TryStreamExt};
use k8s_openapi::NamespaceResourceScope;
use kube_client::api::ListParams;
use kube_client::{Api, Client, Resource};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use stream::BoxStream;

// TODO - Not sure this is the right name?
pub struct SharedStore<K, W = WatcherProvider<K>>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send,
    K::DynamicType: Hash + Eq,
    W: CreateWatcher<K>,
{
    watcher_provider: W,
    reflectors: HashMap<ListParams, (SafeStore<K>, SubscribableBoxStream<K>)>,
}

type SubscribableBoxStream<K> = StreamSubscribable<BoxStream<'static, watcher::Result<Event<K>>>>;

impl<K> SharedStore<K>
where
    K: 'static + Resource + Clone + DeserializeOwned + Debug + Send + Sync,
    K::DynamicType: Default + Eq + Hash + Clone,
{
    pub fn new(api: Api<K>) -> Self {
        Self {
            watcher_provider: WatcherProvider::new(api),
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

    pub fn store(&mut self, list_params: ListParams) -> SafeStore<K> {
        self.reflector(list_params).0
    }

    fn reflector(&mut self, list_params: ListParams) -> (SafeStore<K>, impl Stream<Item = Event<K>>) {
        if let Some((store, prism)) = self.reflectors.get(&list_params) {
            return (store.clone(), prism.subscribe_ok());
        }

        let watcher = self.watcher_provider.watcher(list_params.clone());
        let store_writer = Writer::default();
        let store_reader = store_writer.as_reader();

        let safe_store = SafeStore::new(store_reader);

        // todo - maybe we want a "safe_reflector" ?
        let safe_store_clone = safe_store.clone();
        let reflector = reflector(store_writer, watcher).inspect_ok(move |_| safe_store_clone.make_ready());

        let subscribable_reflector = reflector.boxed().subscribable();
        let event_stream = subscribable_reflector.subscribe_ok();

        self.reflectors
            .insert(list_params.clone(), (safe_store.clone(), subscribable_reflector));

        (safe_store, event_stream)
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
            let mut kp = TestProvider::<Pod>::new(
                hashmap!(lp.clone().into() => vec![Event::Restarted(expected_state.clone())]),
                &ReadyToken::new(),
            );
            let store = kp.store(lp);

            kp.spawn().await;

            assert_eq!(store.cloned_state().await, expected_state);
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

            assert_eq!(store1.cloned_state().await, expected_state);
            assert_eq!(store2.cloned_state().await, expected_state);
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

            assert_eq!(store1.cloned_state().await, expected_state1, "Store 1");
            assert_eq!(store2.cloned_state().await, expected_state2, "Store 2");
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
