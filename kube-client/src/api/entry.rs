//! API helpers for get-or-create and get-and-modify patterns
//!
//! [`Api::entry`] is the primary entry point for this API.

// Import used in docs
#[allow(unused_imports)] use std::collections::HashMap;
use std::fmt::Debug;

use kube_core::{params::PostParams, Resource};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Api, Result};

impl<K: Resource + Clone + DeserializeOwned + Debug> Api<K> {
    /// Gets a given object's "slot" on the Kubernetes API, designed for "get-or-create" and "get-and-modify" patterns
    ///
    /// This is similar to [`HashMap::entry`], but the [`Entry`] must be [`OccupiedEntry::sync`]ed for changes to be persisted.
    ///
    /// # Usage
    ///
    /// ```rust,no_run
    /// # use std::collections::BTreeMap;
    /// # use k8s_openapi::api::core::v1::ConfigMap;
    /// # async fn wrapper() -> Result <(), Box<dyn std::error::Error>> {
    /// let kube = kube::Client::try_default().await?;
    /// let cms = kube::Api::<ConfigMap>::namespaced(kube, "default");
    /// cms
    ///     // Try to get `entry-example` if it exists
    ///     .entry("entry-example").await?
    ///     // Modify object if it already exists
    ///     .and_modify(|cm| {
    ///         cm.data
    ///             .get_or_insert_with(BTreeMap::default)
    ///             .insert("already-exists".to_string(), "true".to_string());
    ///     })
    ///     // Provide a default object if it does not exist
    ///     .or_insert(|| ConfigMap::default())
    ///     // Modify the object unconditionally now that we have provided a default value
    ///     .and_modify(|cm| {
    ///         cm.data
    ///             .get_or_insert_with(BTreeMap::default)
    ///             .insert("modified".to_string(), "true".to_string());
    ///     })
    ///     // Save changes
    ///     .sync().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn entry<'a>(&'a self, name: &'a str) -> Result<Entry<'a, K>> {
        Ok(match self.get_opt(name).await? {
            Some(object) => Entry::Occupied(OccupiedEntry {
                api: self,
                object,
                dirtiness: Dirtiness::Clean,
            }),
            None => Entry::Vacant(VacantEntry { api: self, name }),
        })
    }
}

#[derive(Debug)]
/// A view into a single object, with enough context to create or update it
///
/// See [`Api::entry`] for more information.
pub enum Entry<'a, K> {
    /// An object that either exists on the server, or has been created locally (and is awaiting synchronization)
    Occupied(OccupiedEntry<'a, K>),
    /// An object that does not exist
    Vacant(VacantEntry<'a, K>),
}

impl<'a, K> Entry<'a, K> {
    /// Borrow the object, if it exists (on the API, or queued for creation using [`Entry::or_insert`])
    pub fn get(&self) -> Option<&K> {
        match self {
            Entry::Occupied(entry) => Some(entry.get()),
            Entry::Vacant(_) => None,
        }
    }

    /// Borrow the object mutably, if it exists (on the API, or queued for creation using [`Entry::or_insert`])
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for any changes to be persisted.
    pub fn get_mut(&mut self) -> Option<&mut K> {
        match self {
            Entry::Occupied(entry) => Some(entry.get_mut()),
            Entry::Vacant(_) => None,
        }
    }

    /// Let `f` modify the object, if it exists (on the API, or queued for creation using [`Entry::or_insert`])
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for any changes to be persisted.
    pub fn and_modify(self, f: impl FnOnce(&mut K)) -> Self {
        match self {
            Entry::Occupied(entry) => Entry::Occupied(entry.and_modify(f)),
            entry @ Entry::Vacant(_) => entry,
        }
    }

    /// Create a new object if it does not already exist
    ///
    /// Just like [`VacantEntry::insert`], `name` and `namespace` are automatically set for the new object.
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for the change to be persisted.
    pub fn or_insert(self, default: impl FnOnce() -> K) -> OccupiedEntry<'a, K>
    where
        K: Resource,
    {
        match self {
            Entry::Occupied(entry) => entry,
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }
}

/// A view into a single object that exists
///
/// The object may exist because it existed at the time of call to [`Api::entry`],
/// or because it was created by [`Entry::or_insert`].
#[derive(Debug)]
pub struct OccupiedEntry<'a, K> {
    api: &'a Api<K>,
    dirtiness: Dirtiness,
    object: K,
}

#[derive(Debug)]
enum Dirtiness {
    /// The object has not been modified (locally) since the last API operation
    Clean,
    /// The object exists in the API, but has been modified locally
    Dirty,
    /// The object does not yet exist in the API, but was created locally
    New,
}

impl<'a, K> OccupiedEntry<'a, K> {
    /// Borrow the object
    pub fn get(&self) -> &K {
        &self.object
    }

    /// Borrow the object mutably
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for any changes to be persisted.
    pub fn get_mut(&mut self) -> &mut K {
        self.dirtiness = match self.dirtiness {
            Dirtiness::Clean => Dirtiness::Dirty,
            Dirtiness::Dirty => Dirtiness::Dirty,
            Dirtiness::New => Dirtiness::New,
        };
        &mut self.object
    }

    /// Let `f` modify the object
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for any changes to be persisted.
    pub fn and_modify(mut self, f: impl FnOnce(&mut K)) -> Self {
        f(self.get_mut());
        self
    }

    /// Take ownership over the object
    pub fn into_object(self) -> K {
        self.object
    }

    /// Save the object to the Kubernetes API, if any changes have been made
    ///
    /// The [`OccupiedEntry`] is updated with the new object (including changes made by the API server, such as
    /// `.metadata.resource_version`).
    ///
    /// # Errors
    ///
    /// This function can fail due to transient errors, or due to write conflicts (for example: if another client
    /// created the object between the calls to [`Api::entry`] and [`OccupiedEntry::sync`], or because another
    /// client modified the object in the meantime).
    ///
    /// Any retries should be coarse-grained enough to also include the call to [`Api::entry`], so that the latest
    /// state can be fetched.
    #[tracing::instrument(skip(self))]
    pub async fn sync(&mut self) -> Result<()>
    where
        K: Resource + DeserializeOwned + Serialize + Clone + Debug,
    {
        match self.dirtiness {
            Dirtiness::New => self.object = self.api.create(&PostParams::default(), &self.object).await?,
            Dirtiness::Dirty => {
                self.object = self
                    .api
                    .replace(
                        self.object.meta().name.as_deref().unwrap(),
                        &PostParams::default(),
                        &self.object,
                    )
                    .await?;
            }
            Dirtiness::Clean => (),
        };
        self.dirtiness = Dirtiness::Clean;
        Ok(())
    }
}

/// A view of an object that does not yet exist
///
/// Created by [`Api::entry`], as a variant of [`Entry`]
#[derive(Debug)]
pub struct VacantEntry<'a, K> {
    api: &'a Api<K>,
    name: &'a str,
}

impl<'a, K> VacantEntry<'a, K> {
    /// Create a new object
    ///
    /// `name` and `namespace` are automatically set for the new object, according to the parameters passed to [`Api::entry`].
    ///
    /// [`OccupiedEntry::sync`] must be called afterwards for the change to be persisted.
    #[tracing::instrument(skip(self, object))]
    pub fn insert(self, mut object: K) -> OccupiedEntry<'a, K>
    where
        K: Resource,
    {
        let meta = object.meta_mut();
        match &mut meta.name {
            name @ None => *name = Some(self.name.to_string()),
            Some(name) if name != self.name => {
                tracing::warn!(
                    object.metadata.name = ?name,
                    expected_name = ?self.name,
                    "object's .metadata.name does not match name passed to `Api::entry`"
                )
            }
            Some(_) => (),
        }
        match &mut meta.namespace {
            ns @ None => *ns = self.api.namespace.clone(),
            Some(ns) if Some(ns.as_str()) != self.api.namespace.as_deref() => {
                tracing::warn!(
                    object.metadata.namespace = ?ns,
                    expected_namespace = ?self.api.namespace,
                    "object's .metadata.namespace does not match namespace of `Api`"
                )
            }
            Some(_) => (),
        }
        if meta.generate_name.is_some() {
            tracing::warn!(
                ".metadata.generate_name is set, but is not supported by Entry and will be ignored"
            );
        }
        OccupiedEntry {
            api: self.api,
            object,
            dirtiness: Dirtiness::New,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use k8s_openapi::api::core::v1::ConfigMap;
    use kube_core::{
        params::{DeleteParams, PostParams},
        ErrorResponse, ObjectMeta,
    };

    use crate::{api::entry::Entry, Api, Client, Error};

    #[tokio::test]
    #[ignore] // needs cluster (gets and writes cms)
    async fn entry_create_missing_object() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;
        let api = Api::<ConfigMap>::default_namespaced(client);

        let object_name = "entry-missing-cm";
        if api.get_opt(object_name).await?.is_some() {
            api.delete(object_name, &DeleteParams::default()).await?;
        }

        let entry = api.entry(object_name).await?;
        let entry2 = api.entry(object_name).await?;
        assert_eq!(entry.get(), None);
        assert_eq!(entry2.get(), None);

        // Create object cleanly
        let mut entry = entry.or_insert(|| ConfigMap {
            data: Some([("key".to_string(), "value".to_string())].into()),
            ..ConfigMap::default()
        });
        entry.sync().await?;
        assert_eq!(
            entry
                .get()
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value")
        );
        let fetched_obj = api.get(object_name).await?;
        assert_eq!(
            fetched_obj
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value")
        );

        // Update object
        entry
            .get_mut()
            .data
            .get_or_insert_with(BTreeMap::default)
            .insert("key".to_string(), "value2".to_string());
        entry.sync().await?;
        assert_eq!(
            entry
                .get()
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value2")
        );
        let fetched_obj = api.get(object_name).await?;
        assert_eq!(
            fetched_obj
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value2")
        );

        // Object was already created in parallel, fail with a conflict error
        let mut entry2 = entry2.or_insert(|| ConfigMap {
            data: Some([("key".to_string(), "value3".to_string())].into()),
            ..ConfigMap::default()
        });
        assert!(
            matches!(dbg!(entry2.sync().await), Err(Error::Api(ErrorResponse{reason,..})) if reason == "AlreadyExists")
        );

        // Cleanup
        api.delete(object_name, &DeleteParams::default()).await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (gets and writes cms)
    async fn entry_update_existing_object() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;
        let api = Api::<ConfigMap>::default_namespaced(client);

        let object_name = "entry-existing-cm";
        if api.get_opt(object_name).await?.is_some() {
            api.delete(object_name, &DeleteParams::default()).await?;
        }
        api.create(&PostParams::default(), &ConfigMap {
            metadata: ObjectMeta {
                namespace: api.namespace.clone(),
                name: Some(object_name.to_string()),
                ..ObjectMeta::default()
            },
            data: Some([("key".to_string(), "value".to_string())].into()),
            ..ConfigMap::default()
        })
        .await?;

        let mut entry = match api.entry(object_name).await? {
            Entry::Occupied(entry) => entry,
            entry => panic!("entry for existing object must be occupied: {:?}", entry),
        };
        let mut entry2 = match api.entry(object_name).await? {
            Entry::Occupied(entry) => entry,
            entry => panic!("entry for existing object must be occupied: {:?}", entry),
        };

        // Entry is up-to-date, modify cleanly
        entry
            .get_mut()
            .data
            .get_or_insert_with(BTreeMap::default)
            .insert("key".to_string(), "value2".to_string());
        entry.sync().await?;
        assert_eq!(
            entry
                .get()
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value2")
        );
        let fetched_obj = api.get(object_name).await?;
        assert_eq!(
            fetched_obj
                .data
                .as_ref()
                .and_then(|data| data.get("key"))
                .map(String::as_str),
            Some("value2")
        );

        // Object was already updated in parallel, fail with a conflict error
        entry2
            .get_mut()
            .data
            .get_or_insert_with(BTreeMap::default)
            .insert("key".to_string(), "value3".to_string());
        assert!(
            matches!(entry2.sync().await, Err(Error::Api(ErrorResponse{reason,..})) if reason == "Conflict")
        );

        // Cleanup
        api.delete(object_name, &DeleteParams::default()).await?;
        Ok(())
    }
}
