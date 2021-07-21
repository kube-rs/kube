use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope};

pub trait Scope {
    fn path_segment(&self) -> String;
    fn namespace(&self) -> Option<&str>;
}

pub struct ClusterScope;
pub struct NamespaceScope {
    pub namespace: String,
}

impl Scope for ClusterScope {
    fn path_segment(&self) -> String {
        String::new()
    }

    fn namespace(&self) -> Option<&str> {
        None
    }
}
impl Scope for NamespaceScope {
    fn path_segment(&self) -> String {
        format!("namespaces/{}/", self.namespace)
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }
}

pub trait NativeScope<Kind>: Scope {}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> NativeScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource<Scope = ClusterResourceScope>> NativeScope<Kind> for ClusterScope {}

pub trait ListScope<Kind>: Scope {}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> ListScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource> ListScope<Kind> for ClusterScope {}
