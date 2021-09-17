use futures::TryStreamExt;
use kube::{Api, Resource};
use serde::de::DeserializeOwned;
use snafu::{futures::TryStreamExt as _, Snafu};
use std::fmt::Debug;

use crate::watcher::{self, watch_object};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to probe for whether the condition is fulfilled yet: {}", source))]
    ProbeFailed {
        #[snafu(backtrace)]
        source: watcher::Error,
    },
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
    watch_object(api, name)
        .context(ProbeFailed)
        .try_take_while(|obj| {
            let result = !cond(obj.as_ref());
            async move { Ok(result) }
        })
        .try_for_each(|_| async { Ok(()) })
        .await
}

pub mod conditions {
    use kube::Resource;

    pub fn is_deleted<K: Resource>(uid: &str) -> impl Fn(Option<&K>) -> bool + '_ {
        move |obj: Option<&K>| {
            obj.map_or(
                // Object is not found, success!
                true,
                // Object is found, but a changed uid would mean that it was deleted and recreated
                |obj| obj.meta().uid.as_deref() != Some(uid),
            )
        }
    }
}

pub mod delete {
    use kube::{api::DeleteParams, Api, Resource};
    use serde::de::DeserializeOwned;
    use snafu::{OptionExt, ResultExt, Snafu};
    use std::fmt::Debug;

    use super::{await_condition, conditions};

    #[derive(Snafu, Debug)]
    pub enum Error {
        NoUid,
        Delete { source: kube::Error },
        Await { source: super::Error },
    }

    #[allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]
    pub async fn delete_and_finalize<K: Clone + Debug + Send + DeserializeOwned + Resource + 'static>(
        api: Api<K>,
        name: &str,
        delete_params: &DeleteParams,
    ) -> Result<(), Error> {
        let deleted_obj_uid = api
            .delete(name, delete_params)
            .await
            .context(Delete)?
            .either(
                |mut obj| obj.meta_mut().uid.take(),
                |status| status.details.map(|details| details.uid),
            )
            .context(NoUid)?;
        await_condition(api, name, conditions::is_deleted(&deleted_obj_uid))
            .await
            .context(Await)
    }
}
