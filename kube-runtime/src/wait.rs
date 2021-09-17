use futures::TryStreamExt;
use kube::{api::ListParams, Api, Resource};
use serde::de::DeserializeOwned;
use snafu::{futures::TryStreamExt as _, Backtrace, Snafu};
use std::fmt::Debug;

use crate::watcher;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to probe for whether the condition is fulfilled yet: {}", source))]
    ProbeFailed {
        #[snafu(backtrace)]
        source: watcher::Error,
    },
    #[snafu(display("probe returned invalid response: more than one object matched filter"))]
    ProbeTooManyObjects { backtrace: Backtrace },
}

/// Watch an object, and Wait for some condition `cond` to return `true`.
///
/// `cond` is passed `Some` if the object is found, otherwise `None`.
///
/// # Errors
///
/// Fails if the type is not known to the Kubernetes API, or if the [`Api`] does not have
/// permission to `watch` and `list` it.
///
/// Does *not* fail if the object is not found.
pub async fn await_condition<K>(
    api: Api<K>,
    name: &str,
    mut cond: impl FnMut(Option<&K>) -> bool,
) -> Result<(), Error>
where
    K: Clone + Debug + Send + DeserializeOwned + Resource + 'static,
{
    watcher(api, ListParams {
        field_selector: Some(format!("metadata.name={}", name)),
        ..Default::default()
    })
    .context(ProbeFailed)
    .try_take_while(|event| {
        let obj = match event {
            watcher::Event::Deleted(_) => Ok(None),
            watcher::Event::Restarted(objs) if objs.len() > 1 => ProbeTooManyObjects.fail(),
            watcher::Event::Restarted(objs) => Ok(objs.first()),
            watcher::Event::Applied(_) => Ok(None),
        };
        let result = obj.map(&mut cond);
        async move { result }
    })
    .try_for_each(|_| async { Ok(()) })
    .await
}
