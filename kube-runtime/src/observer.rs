use backoff::ExponentialBackoff;
use futures::Stream;
use kube_client::{
    api::{Api, ListParams},
    core::Resource,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use crate::{
    utils,
    watcher::{backoff_watch, watcher, Error, Event, Result},
};


/// An observer around a watcher with error handling and retry backoff
///
/// # Error handling
///
/// An `Observer` sets a sensible default backoff policy for all watch events and will retry
/// (with expotential backoff) from transient `watcher` failures until the retry policy is breached.
/// The `watcher::Error`s are still returned in the resulting stream, but they __can__ be ignored.
/// If the retry policy is breached, then the stream ends.
///
/// To configure the retry policy use `Observer::backoff`.
///
/// Note that t is possible to create a backoff policy that retries infinitely, but this might be undesirable.
/// Several watch errors represent a need for external user action to recover:
///
/// - 404 `ErrorResponse`(watching invalid / missing api kind/group for `K`)
/// - 403 `ErrorResponse` (missing list + watch rbac verbs for `K`)
pub struct Observer<K>
where
    K: Clone + Resource + Send + Sync + 'static,
{
    // temporary builder params
    api: Api<K>,
    listparams: Option<ListParams>,
    backoff: Option<ExponentialBackoff>,
}

impl<K> Observer<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
{
    /// Create a Observer on a reflector on a type `K`
    ///
    /// Takes an [`Api`] object that determines how the `Observer` listens for changes to the `K`.
    ///
    /// The [`ListParams`] controls to the possible subset of objects of `K` that you want to cache.
    /// For the full set of objects `K` in the given `Api` scope, you can use [`ListParams::default`].
    #[must_use]
    pub fn new(api: Api<K>) -> Self {
        Self {
            api,
            listparams: None,
            backoff: None,
        }
    }

    // start the watcher and filter out backoff errors from the stream for a while
    fn watch_events(self) -> impl Stream<Item = Result<Event<K>>> {
        let backoff = self.backoff.unwrap_or_else(|| backoff::ExponentialBackoff {
            max_elapsed_time: Some(std::time::Duration::from_secs(60 * 60)),
            ..ExponentialBackoff::default()
        });
        let lp = self.listparams.unwrap_or_default();
        let input = watcher(self.api, lp);
        backoff_watch(input, backoff)
    }

    /// Set the backoff policy
    #[must_use]
    pub fn backoff(mut self, backoff: ExponentialBackoff) -> Self {
        // TODO: allow backoff: B where B: Backoff here - needs box_into_inner
        self.backoff = Some(backoff);
        self
    }

    /// Set the parameters for the watch
    #[must_use]
    pub fn params(mut self, lp: ListParams) -> Self {
        self.listparams = Some(lp);
        self
    }

    /// Run the watcher and produce an information stream of watch events (modified/added)
    ///
    /// This stream will emit only `Ok` events until the error policy is breached
    ///
    /// # Errors
    ///
    /// If a [`watcher::Error`] was encountered for longer than what the
    /// [`ExponentialBackoff`](backoff::ExponentialBackoff) policy allows, then
    /// that error is considered irrecoverable and propagated in a stream item here.
    pub fn watch_applies(self) -> impl Stream<Item = Result<K, Error>> {
        utils::try_flatten_applied(self.watch_events())
    }

    /// Run the watcher, and produce an informational stream of watch events (modified/added/deleted)
    ///
    /// This stream will emit only `Ok` events until the error policy is breached
    ///
    /// # Errors
    ///
    /// If a [`watcher::Error`] was encountered for longer than what the
    /// [`ExponentialBackoff`](backoff::ExponentialBackoff) policy allows, then
    /// that error is considered irrecoverable and propagated in a stream item here.
    pub fn watch_touches(self) -> impl Stream<Item = Result<K, Error>> {
        utils::try_flatten_touched(self.watch_events())
    }
}
