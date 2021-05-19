use serde::Deserialize;
// TODO: replace with Status in k8s openapi?

/// A Kubernetes status object
#[allow(missing_docs)]
#[derive(Deserialize, Debug)]
pub struct Status {
    // TODO: typemeta
    // TODO: metadata that can be completely empty (listmeta...)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub code: u16,
}

/// Status details object on the [`Status`] object
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct StatusDetails {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub uid: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<StatusCause>,
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub retry_after_seconds: u32,
}

/// Status cause object on the [`StatusDetails`] object
#[derive(Deserialize, Debug)]
#[allow(missing_docs)]
pub struct StatusCause {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
}


#[cfg(test)]
mod test {
    use super::Status;

    // ensure our status schema is sensible
    #[test]
    fn delete_deserialize_test() {
        let statusresp = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"name":"some-app","group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s: Status = serde_json::from_str::<Status>(statusresp).unwrap();
        assert_eq!(s.details.unwrap().name, "some-app");

        let statusnoname = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s2: Status = serde_json::from_str::<Status>(statusnoname).unwrap();
        assert_eq!(s2.details.unwrap().name, ""); // optional probably better..
    }
}
