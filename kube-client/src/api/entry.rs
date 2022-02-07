#![warn(missing_docs)]

use std::fmt::Debug;

use kube_core::{params::PostParams, Resource};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Api, Result};

impl<K: Resource + Clone + DeserializeOwned + Debug> Api<K> {
    pub async fn entry<'a>(&'a self, name: &'a str) -> Result<Entry<'a, K>> {
        Ok(match self.try_get(name).await? {
            Some(object) => Entry::Occupied(OccupiedEntry {
                api: self,
                object,
                new: false,
            }),
            None => Entry::Vacant(VacantEntry { api: self, name }),
        })
    }
}

#[derive(Debug)]
pub enum Entry<'a, K> {
    Occupied(OccupiedEntry<'a, K>),
    Vacant(VacantEntry<'a, K>),
}

impl<'a, K> Entry<'a, K> {
    pub fn get(&self) -> Option<&K> {
        match self {
            Entry::Occupied(entry) => Some(entry.get()),
            Entry::Vacant(_) => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut K> {
        match self {
            Entry::Occupied(entry) => Some(entry.get_mut()),
            Entry::Vacant(_) => None,
        }
    }

    pub fn and_modify(self, f: impl FnOnce(&mut K)) -> Self {
        match self {
            Entry::Occupied(entry) => Entry::Occupied(entry.and_modify(f)),
            entry @ Entry::Vacant(_) => entry,
        }
    }

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

pub struct OccupiedEntry<'a, K> {
    api: &'a Api<K>,
    new: bool,
    object: K,
}

impl<'a, K> OccupiedEntry<'a, K> {
    pub fn get(&self) -> &K {
        &self.object
    }

    pub fn get_mut(&mut self) -> &mut K {
        &mut self.object
    }

    pub fn and_modify(mut self, f: impl FnOnce(&mut K)) -> Self {
        f(&mut self.object);
        self
    }

    pub fn into_object(self) -> K {
        self.object
    }

    pub async fn sync(&mut self) -> Result<()>
    where
        K: Resource + DeserializeOwned + Serialize + Clone + Debug,
    {
        if self.new {
            self.new = false;
            self.object = self.api.create(&PostParams::default(), &self.object).await?;
            Ok(())
        } else {
            self.object = self
                .api
                .replace(
                    self.object.meta().name.as_deref().unwrap(),
                    &PostParams::default(),
                    &self.object,
                )
                .await?;
            Ok(())
        }
    }
}

pub struct VacantEntry<'a, K> {
    api: &'a Api<K>,
    name: &'a str,
}

impl<'a, K> VacantEntry<'a, K> {
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
            new: true,
        }
    }
}

impl<'a, K: Debug> Debug for OccupiedEntry<'a, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("api", &"...")
            .field("new", &self.new)
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
