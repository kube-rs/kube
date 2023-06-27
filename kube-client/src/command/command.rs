use k8s_openapi::serde_value::Value;
use kube_core::params::{DeleteParams, Patch, PatchParams, PostParams};
use kube_core::{ApiResource, DynamicObject, Resource};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::option::Option::None;

/// TODO
#[derive(Clone, Debug, PartialEq)]
pub struct KubeCommand {
    /// TODO
    pub namespace: Option<String>,
    /// TODO
    pub verb: KubeCommandVerb,
}

impl KubeCommand {
    /// TODO
    pub fn cluster(command: KubeCommandVerb) -> KubeCommand {
        Self {
            namespace: None,
            verb: command,
        }
    }

    /// TODO
    pub fn namespaced(namespace: &str, command: KubeCommandVerb) -> KubeCommand {
        Self {
            namespace: Some(namespace.to_string()),
            verb: command,
        }
    }

    /// TODO
    pub fn api_resource(&self) -> ApiResource {
        self.verb.api_resource()
    }

    /// TODO
    pub fn kind(&self) -> String {
        self.verb.kind()
    }

    /// TODO
    pub fn name(&self) -> String {
        self.verb.name()
    }

    /// TODO
    pub fn namespace(&self) -> Option<String> {
        self.namespace.clone()
    }

    /// TODO
    pub fn verb_name(&self) -> String {
        self.verb.verb_name()
    }
}

/// TODO
#[derive(Clone, Debug, PartialEq)]
pub enum KubeCommandVerb {
    Create {
        name: String,
        object: Box<DynamicObject>,
        resource: ApiResource,
        params: PostParams,
    },
    Replace {
        name: String,
        object: Box<DynamicObject>,
        resource: ApiResource,
        params: PostParams,
    },
    ReplaceStatus {
        name: String,
        data: Vec<u8>,
        resource: ApiResource,
        params: PostParams,
    },
    Patch {
        name: String,
        patch: Patch<Value>,
        resource: ApiResource,
        params: PatchParams,
    },
    PatchStatus {
        name: String,
        patch: Patch<Value>,
        resource: ApiResource,
        params: PatchParams,
    },
    Delete {
        name: String,
        resource: ApiResource,
        params: DeleteParams,
    },
}

impl Display for KubeCommandVerb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}: {}/{}", self.verb_name(), self.kind(), self.name()))
    }
}

impl KubeCommandVerb {
    pub fn create<K: Resource + Serialize>(
        name: String,
        resource: K,
        params: PostParams,
    ) -> Result<KubeCommandVerb, serde_json::Error>
    where
        K::DynamicType: Default,
    {
        let mut dynamic_object = DynamicObject::new(&name, &ApiResource::erase::<K>(&Default::default()));

        dynamic_object.metadata = resource.meta().clone();
        dynamic_object.data = serde_json::to_value(resource)?;

        Ok(KubeCommandVerb::Create {
            name,
            object: Box::new(dynamic_object),
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        })
    }

    pub fn replace<K: Resource + Serialize>(
        name: String,
        resource: K,
        params: PostParams,
    ) -> Result<KubeCommandVerb, serde_json::Error>
    where
        K::DynamicType: Default,
    {
        let mut dynamic_object = DynamicObject::new(&name, &ApiResource::erase::<K>(&Default::default()));

        dynamic_object.metadata = resource.meta().clone();
        dynamic_object.data = serde_json::to_value(resource)?;

        Ok(KubeCommandVerb::Replace {
            name,
            object: Box::new(dynamic_object),
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        })
    }

    pub fn replace_status<K: Resource + Serialize>(
        name: String,
        resource: K,
        params: PostParams,
    ) -> Result<KubeCommandVerb, serde_json::Error>
    where
        K::DynamicType: Default,
    {
        Ok(KubeCommandVerb::ReplaceStatus {
            name,
            data: serde_json::to_vec(&resource)?,
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        })
    }

    pub fn patch<K: Resource>(name: String, patch: Patch<Value>, params: PatchParams) -> KubeCommandVerb
    where
        K::DynamicType: Default,
    {
        KubeCommandVerb::Patch {
            name,
            patch: patch,
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        }
    }

    pub fn patch_status<K: Resource>(
        name: String,
        patch: Patch<Value>,
        params: PatchParams,
    ) -> KubeCommandVerb
    where
        K::DynamicType: Default,
    {
        KubeCommandVerb::PatchStatus {
            name,
            patch,
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        }
    }

    pub fn delete<K: Resource>(name: String, params: DeleteParams) -> KubeCommandVerb
    where
        K::DynamicType: Default,
    {
        KubeCommandVerb::Delete {
            name,
            resource: ApiResource::erase::<K>(&Default::default()),
            params,
        }
    }

    pub fn in_scope(self, namespace: Option<String>) -> KubeCommand {
        KubeCommand {
            namespace,
            verb: self,
        }
    }

    pub fn in_cluster(self) -> KubeCommand {
        KubeCommand {
            namespace: None,
            verb: self,
        }
    }

    pub fn in_namespace(self, namespace: String) -> KubeCommand {
        KubeCommand {
            namespace: Some(namespace),
            verb: self,
        }
    }

    pub fn name(&self) -> String {
        match self {
            KubeCommandVerb::Create { name, .. }
            | KubeCommandVerb::Replace { name, .. }
            | KubeCommandVerb::ReplaceStatus { name, .. }
            | KubeCommandVerb::Patch { name, .. }
            | KubeCommandVerb::PatchStatus { name, .. }
            | KubeCommandVerb::Delete { name, .. } => name.to_string(),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            KubeCommandVerb::Create { resource, .. }
            | KubeCommandVerb::Replace { resource, .. }
            | KubeCommandVerb::ReplaceStatus { resource, .. }
            | KubeCommandVerb::Patch { resource, .. }
            | KubeCommandVerb::PatchStatus { resource, .. }
            | KubeCommandVerb::Delete { resource, .. } => resource.kind.to_string(),
        }
    }

    pub fn verb_name(&self) -> String {
        match self {
            KubeCommandVerb::Create { .. } => "Create".to_string(),
            KubeCommandVerb::Replace { .. } => "Replace".to_string(),
            KubeCommandVerb::ReplaceStatus { .. } => "ReplaceStatus".to_string(),
            KubeCommandVerb::Patch { .. } => "Patch".to_string(),
            KubeCommandVerb::PatchStatus { .. } => "PatchStatus".to_string(),
            KubeCommandVerb::Delete { .. } => "Delete".to_string(),
        }
    }

    pub fn api_resource(&self) -> ApiResource {
        match self {
            KubeCommandVerb::Create { resource, .. }
            | KubeCommandVerb::Replace { resource, .. }
            | KubeCommandVerb::ReplaceStatus { resource, .. }
            | KubeCommandVerb::Patch { resource, .. }
            | KubeCommandVerb::PatchStatus { resource, .. }
            | KubeCommandVerb::Delete { resource, .. } => resource.clone(),
        }
    }
}
