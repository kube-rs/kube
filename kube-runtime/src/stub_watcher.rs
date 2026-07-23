use futures::{FutureExt, StreamExt};
use kube_client::{
    Resource,
    api::{ListParams, ObjectList, TypeMeta, WatchEvent, WatchParams},
};
use serde::de::DeserializeOwned;
use std::{
    cell::{Ref, RefCell},
    fmt::Debug,
    pin::Pin,
};

use crate::watcher::ApiMode;

pub enum Recording {
    List(ListParams),
    Watch(WatchParams, String),
}

/// `TestMode` is the test-only "mock" implementation for [`ApiMode`].
///
/// [`TestMode::fixture`] is the fixed list of values `TestMode` returns, removing
/// one element per call and returning it once none are left.
/// This enables us to simulate different list scenarios.
///
/// [`TestMode::watch_sequence`] is the fixed list of [`Sequence`]s `TestMode` returns. The
/// behaviour, is similar to [`TestMode::fixture`] but it allows for simulating "waiting" periods,
/// empty intermediary result and for returning a [`futures::stream::BoxStream`] implemented via [`TestStream`].
pub struct TestMode<K>
where
    K: Clone + Debug + DeserializeOwned + Send + 'static,
{
    list_sequence: RefCell<Vec<kube_client::Result<ObjectList<K>>>>,
    watch_sequence: RefCell<Vec<Sequence<K>>>,
    recorder: RefCell<Vec<Recording>>,
}

impl<K> TestMode<K>
where
    K: Clone + Debug + DeserializeOwned + Send,
{
    pub fn get_recordings(&self) -> Ref<'_, Vec<Recording>> {
        self.recorder.borrow()
    }
}

impl<K> TestMode<K>
where
    K: Clone + Debug + DeserializeOwned + Send,
{
    /// Arguments are mut because we reverse the order internally because we pop off the end
    /// which means the first element to be returned would the last which would be unexpected.
    pub fn new(
        mut fixture: Vec<kube_client::Result<ObjectList<K>>>,
        mut watch_sequence: Vec<Sequence<K>>,
    ) -> Self {
        fixture.reverse();
        watch_sequence.reverse();
        Self {
            list_sequence: RefCell::new(fixture),
            watch_sequence: RefCell::new(watch_sequence),
            recorder: RefCell::new(vec![]),
        }
    }
}

pub struct ResultPage<K>
where
    K: Clone,
{
    inner: ObjectList<K>,
}

impl<K> ResultPage<K>
where
    K: Clone + Resource<DynamicType = ()>,
{
    pub fn empty() -> Self {
        ResultPage { inner: empty_list() }
    }

    pub fn continue_token(mut self, token: Option<&str>) -> Self {
        self.inner.metadata.continue_ = token.map(str::to_string);
        self
    }

    pub fn resource_version(mut self, version: Option<&str>) -> Self {
        self.inner.metadata.resource_version = version.map(str::to_string);
        self
    }

    pub fn items(mut self, items: Vec<K>) -> Self {
        self.inner.items = items;
        self
    }
}

impl<K> From<ResultPage<K>> for ObjectList<K>
where
    K: Clone + Resource<DynamicType = ()>,
{
    fn from(value: ResultPage<K>) -> Self {
        value.inner
    }
}

fn empty_list<K>() -> ObjectList<K>
where
    K: Clone + Resource<DynamicType = ()>,
{
    ObjectList {
        types: TypeMeta::list::<K>(),
        metadata: kube_client::api::ListMeta::default(),
        items: Vec::new(),
    }
}

/// Utility enum to represent different "Segments" of a continuum over repeated calls on a running watch(er).
pub enum Sequence<K> {
    /// Represents end of stream, i.e., [`std::task::Poll::Ready`] with [`None`]
    Terminate,
    /// Represents returning from a list of results until the inner list is empty, i.e., [`std::task::Poll::Ready`] with one [`kube_client::Result<WatchEvent<_>>`]
    /// for each call.
    List(Vec<kube_client::Result<WatchEvent<K>>>),
    /// Represents a "sleep"/wait behaviour to simulate a watch(er) not returning elements for a
    /// certain duration.
    Wait(std::time::Duration),
}

/// Implements [`futures::stream::BoxStream`] via [`futures::Stream`] for internal use via [`TestMode::watch`]
pub struct TestStream<K> {
    seq: RefCell<Vec<Sequence<K>>>,
    waiting: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl<K: Unpin> futures::Stream for TestStream<K> {
    type Item = kube_client::Result<WatchEvent<K>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(inner_fut) = this.waiting.as_mut() {
            if !matches!(inner_fut.poll_unpin(cx), std::task::Poll::Ready(())) {
                return std::task::Poll::Pending;
            }
            this.waiting = None;
        }
        let mut inner = this.seq.borrow_mut();
        match inner.pop() {
            Some(seq) => match seq {
                Sequence::Terminate => std::task::Poll::Ready(None),
                Sequence::List(mut watch_events) => std::task::Poll::Ready(watch_events.pop()),
                Sequence::Wait(duration) => {
                    if this.waiting.is_some() {
                        unreachable!("TestStream::waiting should be None when accessing inner, this is a bug")
                    }
                    this.waiting = Some(Box::pin(tokio::time::sleep(duration)));
                    std::task::Poll::Pending
                }
            },
            None => std::task::Poll::Ready(None),
        }
    }
}

#[allow(clippy::unused_async_trait_impl)]
impl<K> ApiMode for TestMode<K>
where
    K: Resource<DynamicType = ()> + Clone + Debug + DeserializeOwned + Send + Unpin + 'static,
{
    type Value = K;

    async fn list(&self, lp: &ListParams) -> kube_client::Result<ObjectList<Self::Value>> {
        self.recorder.borrow_mut().push(Recording::List(lp.clone()));
        match self.list_sequence.borrow_mut().pop() {
            Some(next) => next,
            None => Ok(empty_list()),
        }
    }

    async fn watch(
        &self,
        wp: &WatchParams,
        version: &str,
    ) -> kube_client::Result<futures::stream::BoxStream<'static, kube_client::Result<WatchEvent<Self::Value>>>>
    {
        self.recorder
            .borrow_mut()
            .push(Recording::Watch(wp.clone(), version.into()));
        let seq = self.watch_sequence.borrow_mut().pop().into_iter().collect();
        Ok(TestStream {
            seq: RefCell::new(seq),
            waiting: None,
        }
        .fuse()
        .boxed())
    }
}
