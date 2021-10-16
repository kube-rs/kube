use std::convert::TryFrom;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The name of the controller instance publishing the event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::event_recorder::InstanceName;
///
/// let instance_name: InstanceName = "my-awesome-controller-abcdef".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - shorter than 128 characters.
pub struct InstanceName(String);

impl TryFrom<&str> for InstanceName {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for InstanceName {
    type Error = String;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        let n_chars = v.chars().count();
        if n_chars > 128 {
            Err(format!(
                "The reporting instance name must be shorter than 128 characters.\n{} is {} characters long.",
                v, n_chars
            ))
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for InstanceName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for InstanceName {
    fn into(self) -> String {
        self.0
    }
}
