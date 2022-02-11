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
    /// This is similar to [`HashMap::entry`], but [`Entry`] must be [`Entry::sync`]ed for changes to be persisted.
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
/// See [`Api::entry`].
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
    /// [`Entry::sync`] must be called afterwards for any changes to be persisted.
    pub fn get_mut(&mut self) -> Option<&mut K> {
        match self {
            Entry::Occupied(entry) => Some(entry.get_mut()),
            Entry::Vacant(_) => None,
        }
    }

    /// Let `f` modify the object, if it exists (on the API, or queued for creation using [`Entry::or_insert`])
    ///
    /// [`Entry::sync`] must be called afterwards for any changes to be persisted.
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
    /// [`Entry::sync`] must be called afterwards for any changes to be persisted.
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
    /// [`Entry::sync`] must be called afterwards for any changes to be persisted.
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
    pub async fn sync(&mut self) -> Result<()>
    where
        K: Resource + DeserializeOwned + Serialize + Clone + Debug,
    {
        self.object = match self.dirtiness {
            Dirtiness::New => self.api.create(&PostParams::default(), &self.object).await?,
            Dirtiness::Dirty => {
                self.api
                    .replace(
                        self.object.meta().name.as_deref().unwrap(),
                        &PostParams::default(),
                        &self.object,
                    )
                    .await?
            }
            Dirtiness::Clean => self.api.get(self.object.meta().name.as_deref().unwrap()).await?,
        };
        self.dirtiness = Dirtiness::Clean;
        Ok(())
    }
}

/// A view of an object that does not yet exist
///
/// Created by [`Api::entry`], as a variant of [`Entry`]
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
    pub fn insert(self, mut object: K) -> OccupiedEntry<'a, K>
    where
        K: Resource,
    {
        let meta = object.meta_mut();
        meta.name.get_or_insert_with(|| self.name.to_string());
        if meta.namespace.is_none() {
            meta.namespace = self.api.namespace.clone();
        }
        OccupiedEntry {
            api: self.api,
            object,
            dirtiness: Dirtiness::New,
        }
    }
}

impl<'a, K: Debug> Debug for OccupiedEntry<'a, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("api", &"...")
            .field("dirtiness", &self.dirtiness)
            .field("object", &self.object)
            .finish()
    }
}

impl<'a, K> Debug for VacantEntry<'a, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VacantEntry")
            .field("api", &"...")
            .field("name", &self.name)
            .field("namespace", &self.api.namespace)
            .finish()
    }
}
