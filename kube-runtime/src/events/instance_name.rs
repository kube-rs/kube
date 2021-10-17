use std::{convert::TryFrom, fmt::Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The name of the controller pod publishing the event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::ControllerPodName;
///
/// let controller_pod_name: ControllerPodName = "my-awesome-controller-abcdef".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - shorter than 128 characters.
pub struct ControllerPodName(String);

impl TryFrom<&str> for ControllerPodName {
    type Error = ControllerPodNameParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for ControllerPodName {
    type Error = ControllerPodNameParsingError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        let n_chars = v.chars().count();
        if n_chars > 128 {
            Err(ControllerPodNameParsingError { controller_pod_name: v })
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for ControllerPodName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for ControllerPodName {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ControllerPodNameParsingError {
    controller_pod_name: String,
}

impl std::fmt::Display for ControllerPodNameParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "The controller pod name must be shorter than 128 characters.")
    }
}

impl std::error::Error for ControllerPodNameParsingError {}
