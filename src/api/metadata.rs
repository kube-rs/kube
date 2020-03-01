pub use k8s_openapi::Metadata;
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};


#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    ///
    /// Marked optional because it's not always present for items in a `ResourceList`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// The name of the API
    ///
    /// Marked optional because it's not always present for items in a `ResourceList`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

pub trait MetaContent: Metadata {
    fn resource_ver(&self) -> Option<String>;
    fn name(&self) -> String;
    fn namespace(&self) -> Option<String>;
}

/// Any main Kind that is not a listable should use ObjectMeta
impl<K> MetaContent for K
where
    K: Metadata<Ty = ObjectMeta>,
{
    fn resource_ver(&self) -> Option<String> {
        self.metadata()
            .expect("all useful k8s_openapi types have metadata")
            .resource_version
            .clone()
    }

    fn name(&self) -> String {
        self.metadata()
            .expect("all useful k8s_openapi types have metadata")
            .name
            .clone()
            .unwrap()
    }

    fn namespace(&self) -> Option<String> {
        self.metadata()
            .expect("all useful k8s_openapi types have metadata")
            .namespace
            .clone()
    }
}

/*/// Status is a return value for calls that don't return other objects
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Status {
    pub code: Option<i32>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
}
*/
