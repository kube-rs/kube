use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{KubeconfigError, LoadDataError};

/// [`Kubeconfig`] represents information on how to connect to a remote Kubernetes cluster
///
/// Stored in `~/.kube/config` by default, but can be distributed across multiple paths in passed through `KUBECONFIG`.
/// An analogue of the [config type from client-go](https://github.com/kubernetes/client-go/blob/7697067af71046b18e03dbda04e01a5bb17f9809/tools/clientcmd/api/types.go).
///
/// This type (and its children) are exposed primarily for convenience.
///
/// [`Config`][crate::Config] is the __intended__ developer interface to help create a [`Client`][crate::Client],
/// and this will handle the difference between in-cluster deployment and local development.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Kubeconfig {
    /// General information to be use for cli interactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Preferences>,
    /// Referencable names to cluster configs
    pub clusters: Vec<NamedCluster>,
    /// Referencable names to user configs
    #[serde(rename = "users")]
    pub auth_infos: Vec<NamedAuthInfo>,
    /// Referencable names to context configs
    pub contexts: Vec<NamedContext>,
    /// The name of the context that you would like to use by default
    #[serde(rename = "current-context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_context: Option<String>,
    /// Additional information for extenders so that reads and writes don't clobber unknown fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<NamedExtension>>,

    // legacy fields TODO: remove
    /// Legacy field from TypeMeta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Legacy field from TypeMeta
    #[serde(rename = "apiVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// Preferences stores extensions for cli.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Preferences {
    /// Enable colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<bool>,
    /// Extensions holds additional information. This is useful for extenders so that reads and writes don't clobber unknown fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<NamedExtension>>,
}

/// NamedExtention associates name with extension.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NamedExtension {
    /// Name of extension
    pub name: String,
    /// Additional information for extenders so that reads and writes don't clobber unknown fields
    pub extension: serde_json::Value,
}

/// NamedCluster associates name with cluster.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NamedCluster {
    /// Name of cluster
    pub name: String,
    /// Information about how to communicate with a kubernetes cluster
    pub cluster: Cluster,
}

/// Cluster stores information to connect Kubernetes cluster.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Cluster {
    /// The address of the kubernetes cluster (https://hostname:port).
    pub server: String,
    /// Skips the validity check for the server's certificate. This will make your HTTPS connections insecure.
    #[serde(rename = "insecure-skip-tls-verify")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure_skip_tls_verify: Option<bool>,
    /// The path to a cert file for the certificate authority.
    #[serde(rename = "certificate-authority")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_authority: Option<String>,
    /// PEM-encoded certificate authority certificates. Overrides `certificate_authority`
    #[serde(rename = "certificate-authority-data")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_authority_data: Option<String>,
    /// URL to the proxy to be used for all requests.
    #[serde(rename = "proxy-url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// Additional information for extenders so that reads and writes don't clobber unknown fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<NamedExtension>>,
}

/// NamedAuthInfo associates name with authentication.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct NamedAuthInfo {
    /// Name of the user
    pub name: String,
    /// Information that describes identity of the user
    #[serde(rename = "user")]
    pub auth_info: AuthInfo,
}

fn serialize_secretstring<S>(pw: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match pw {
        Some(secret) => serializer.serialize_str(secret.expose_secret()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_secretstring<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(secret) => Ok(Some(SecretString::new(secret))),
        Err(e) => Err(e),
    }
}

/// AuthInfo stores information to tell cluster who you are.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthInfo {
    /// The username for basic authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// The password for basic authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[serde(
        serialize_with = "serialize_secretstring",
        deserialize_with = "deserialize_secretstring"
    )]
    pub password: Option<SecretString>,

    /// The bearer token for authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[serde(
        serialize_with = "serialize_secretstring",
        deserialize_with = "deserialize_secretstring"
    )]
    pub token: Option<SecretString>,
    /// Pointer to a file that contains a bearer token (as described above). If both `token` and token_file` are present, `token` takes precedence.
    #[serde(rename = "tokenFile")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_file: Option<String>,

    /// Path to a client cert file for TLS.
    #[serde(rename = "client-certificate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate: Option<String>,
    /// PEM-encoded data from a client cert file for TLS. Overrides `client_certificate`
    #[serde(rename = "client-certificate-data")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_data: Option<String>,

    /// Path to a client key file for TLS.
    #[serde(rename = "client-key")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    /// PEM-encoded data from a client key file for TLS. Overrides `client_key`
    #[serde(rename = "client-key-data")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[serde(
        serialize_with = "serialize_secretstring",
        deserialize_with = "deserialize_secretstring"
    )]
    pub client_key_data: Option<SecretString>,

    /// The username to act-as.
    #[serde(rename = "as")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonate: Option<String>,
    /// The groups to imperonate.
    #[serde(rename = "as-groups")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonate_groups: Option<Vec<String>>,

    /// Specifies a custom authentication plugin for the kubernetes cluster.
    #[serde(rename = "auth-provider")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider: Option<AuthProviderConfig>,

    /// Specifies a custom exec-based authentication plugin for the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<ExecConfig>,
}

#[cfg(test)]
impl PartialEq for AuthInfo {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_value(self).unwrap() == serde_json::to_value(other).unwrap()
    }
}

/// AuthProviderConfig stores auth for specified cloud provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct AuthProviderConfig {
    /// Name of the auth provider
    pub name: String,
    /// Auth provider configuration
    pub config: HashMap<String, String>,
}

/// ExecConfig stores credential-plugin configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct ExecConfig {
    /// Preferred input version of the ExecInfo.
    ///
    /// The returned ExecCredentials MUST use the same encoding version as the input.
    #[serde(rename = "apiVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Command to execute.
    pub command: String,
    /// Arguments to pass to the command when executing it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Env defines additional environment variables to expose to the process.
    ///
    /// TODO: These are unioned with the host's environment, as well as variables client-go uses to pass argument to the plugin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<HashMap<String, String>>>,
}

/// NamedContext associates name with context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NamedContext {
    /// Name of the context
    pub name: String,
    /// Associations for the context
    pub context: Context,
}

/// Context stores tuple of cluster and user information.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Context {
    /// Name of the cluster for this context
    pub cluster: String,
    /// Name of the `AuthInfo` for this context
    pub user: String,
    /// The default namespace to use on unspecified requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Additional information for extenders so that reads and writes don't clobber unknown fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<NamedExtension>>,
}

const KUBECONFIG: &str = "KUBECONFIG";

/// Some helpers on the raw Config object are exposed for people needing to parse it
impl Kubeconfig {
    /// Read a Config from an arbitrary location
    pub fn read_from<P: AsRef<Path>>(path: P) -> Result<Kubeconfig, KubeconfigError> {
        let data = fs::read_to_string(&path)
            .map_err(|source| KubeconfigError::ReadConfig(source, path.as_ref().into()))?;

        // Remap all files we read to absolute paths.
        let mut merged_docs = None;
        for mut config in kubeconfig_from_yaml(&data)? {
            if let Some(dir) = path.as_ref().parent() {
                for named in config.clusters.iter_mut() {
                    if let Some(path) = &named.cluster.certificate_authority {
                        if let Some(abs_path) = to_absolute(dir, path) {
                            named.cluster.certificate_authority = Some(abs_path);
                        }
                    }
                }
                for named in config.auth_infos.iter_mut() {
                    if let Some(path) = &named.auth_info.client_certificate {
                        if let Some(abs_path) = to_absolute(dir, path) {
                            named.auth_info.client_certificate = Some(abs_path);
                        }
                    }
                    if let Some(path) = &named.auth_info.client_key {
                        if let Some(abs_path) = to_absolute(dir, path) {
                            named.auth_info.client_key = Some(abs_path);
                        }
                    }
                    if let Some(path) = &named.auth_info.token_file {
                        if let Some(abs_path) = to_absolute(dir, path) {
                            named.auth_info.token_file = Some(abs_path);
                        }
                    }
                }
            }
            if let Some(c) = merged_docs {
                merged_docs = Some(Kubeconfig::merge(c, config)?);
            } else {
                merged_docs = Some(config);
            }
        }
        // Empty file defaults to an empty Kubeconfig
        Ok(merged_docs.unwrap_or_default())
    }

    /// Read a Config from an arbitrary YAML string
    ///
    /// This is preferable to using serde_yaml::from_str() because it will correctly
    /// parse multi-document YAML text and merge them into a single `Kubeconfig`
    pub fn from_yaml(text: &str) -> Result<Kubeconfig, KubeconfigError> {
        kubeconfig_from_yaml(text)?
            .into_iter()
            .try_fold(Kubeconfig::default(), Kubeconfig::merge)
    }

    /// Read a Config from `KUBECONFIG` or the the default location.
    pub fn read() -> Result<Kubeconfig, KubeconfigError> {
        match Self::from_env()? {
            Some(config) => Ok(config),
            None => Self::read_from(default_kube_path().ok_or(KubeconfigError::FindPath)?),
        }
    }

    /// Create `Kubeconfig` from `KUBECONFIG` environment variable.
    /// Supports list of files to be merged.
    ///
    /// # Panics
    ///
    /// Panics if `KUBECONFIG` value contains the NUL character.
    pub fn from_env() -> Result<Option<Self>, KubeconfigError> {
        match std::env::var_os(KUBECONFIG) {
            Some(value) => {
                let paths = std::env::split_paths(&value)
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect::<Vec<_>>();
                if paths.is_empty() {
                    return Ok(None);
                }

                let merged = paths.iter().try_fold(Kubeconfig::default(), |m, p| {
                    Kubeconfig::read_from(p).and_then(|c| m.merge(c))
                })?;
                Ok(Some(merged))
            }

            None => Ok(None),
        }
    }

    /// Merge kubeconfig file according to the rules described in
    /// <https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/#merging-kubeconfig-files>
    ///
    /// > Merge the files listed in the `KUBECONFIG` environment variable according to these rules:
    /// >
    /// > - Ignore empty filenames.
    /// > - Produce errors for files with content that cannot be deserialized.
    /// > - The first file to set a particular value or map key wins.
    /// > - Never change the value or map key.
    /// >   Example: Preserve the context of the first file to set `current-context`.
    /// >   Example: If two files specify a `red-user`, use only values from the first file's `red-user`.
    /// >            Even if the second file has non-conflicting entries under `red-user`, discard them.
    fn merge(mut self, next: Kubeconfig) -> Result<Self, KubeconfigError> {
        if self.kind.is_some() && next.kind.is_some() && self.kind != next.kind {
            return Err(KubeconfigError::KindMismatch);
        }
        if self.api_version.is_some() && next.api_version.is_some() && self.api_version != next.api_version {
            return Err(KubeconfigError::ApiVersionMismatch);
        }

        self.kind = self.kind.or(next.kind);
        self.api_version = self.api_version.or(next.api_version);
        self.preferences = self.preferences.or(next.preferences);
        append_new_named(&mut self.clusters, next.clusters, |x| &x.name);
        append_new_named(&mut self.auth_infos, next.auth_infos, |x| &x.name);
        append_new_named(&mut self.contexts, next.contexts, |x| &x.name);
        self.current_context = self.current_context.or(next.current_context);
        self.extensions = self.extensions.or(next.extensions);
        Ok(self)
    }
}

fn kubeconfig_from_yaml(text: &str) -> Result<Vec<Kubeconfig>, KubeconfigError> {
    let mut documents = vec![];
    for doc in serde_yaml::Deserializer::from_str(text) {
        let value = serde_yaml::Value::deserialize(doc).map_err(KubeconfigError::Parse)?;
        let kubeconfig = serde_yaml::from_value(value).map_err(KubeconfigError::InvalidStructure)?;
        documents.push(kubeconfig);
    }
    Ok(documents)
}

#[allow(clippy::redundant_closure)]
fn append_new_named<T, F>(base: &mut Vec<T>, next: Vec<T>, f: F)
where
    F: Fn(&T) -> &String,
{
    use std::collections::HashSet;
    base.extend({
        let existing = base.iter().map(|x| f(x)).collect::<HashSet<_>>();
        next.into_iter()
            .filter(|x| !existing.contains(f(x)))
            .collect::<Vec<_>>()
    });
}

fn to_absolute(dir: &Path, file: &str) -> Option<String> {
    let path = Path::new(&file);
    if path.is_relative() {
        dir.join(path).to_str().map(str::to_owned)
    } else {
        None
    }
}

impl Cluster {
    pub(crate) fn load_certificate_authority(&self) -> Result<Option<Vec<u8>>, KubeconfigError> {
        if self.certificate_authority.is_none() && self.certificate_authority_data.is_none() {
            return Ok(None);
        }

        let ca = load_from_base64_or_file(
            &self.certificate_authority_data.as_deref(),
            &self.certificate_authority,
        )
        .map_err(KubeconfigError::LoadCertificateAuthority)?;
        Ok(Some(ca))
    }
}

impl AuthInfo {
    pub(crate) fn identity_pem(&self) -> Result<Vec<u8>, KubeconfigError> {
        let client_cert = &self.load_client_certificate()?;
        let client_key = &self.load_client_key()?;
        let mut buffer = client_key.clone();
        buffer.extend_from_slice(client_cert);
        Ok(buffer)
    }

    pub(crate) fn load_client_certificate(&self) -> Result<Vec<u8>, KubeconfigError> {
        // TODO Shouldn't error when `self.client_certificate_data.is_none() && self.client_certificate.is_none()`

        load_from_base64_or_file(&self.client_certificate_data.as_deref(), &self.client_certificate)
            .map_err(KubeconfigError::LoadClientCertificate)
    }

    pub(crate) fn load_client_key(&self) -> Result<Vec<u8>, KubeconfigError> {
        // TODO Shouldn't error when `self.client_key_data.is_none() && self.client_key.is_none()`

        load_from_base64_or_file(
            &self
                .client_key_data
                .as_ref()
                .map(|secret| secret.expose_secret().as_str()),
            &self.client_key,
        )
        .map_err(KubeconfigError::LoadClientKey)
    }
}

fn load_from_base64_or_file<P: AsRef<Path>>(
    value: &Option<&str>,
    file: &Option<P>,
) -> Result<Vec<u8>, LoadDataError> {
    let data = value
        .map(load_from_base64)
        .or_else(|| file.as_ref().map(load_from_file))
        .unwrap_or_else(|| Err(LoadDataError::NoBase64DataOrFile))?;
    Ok(ensure_trailing_newline(data))
}

fn load_from_base64(value: &str) -> Result<Vec<u8>, LoadDataError> {
    base64::decode(value).map_err(LoadDataError::DecodeBase64)
}

fn load_from_file<P: AsRef<Path>>(file: &P) -> Result<Vec<u8>, LoadDataError> {
    fs::read(file).map_err(|source| LoadDataError::ReadFile(source, file.as_ref().into()))
}

// Ensure there is a trailing newline in the blob
// Don't bother if the blob is empty
fn ensure_trailing_newline(mut data: Vec<u8>) -> Vec<u8> {
    if data.last().map(|end| *end != b'\n').unwrap_or(false) {
        data.push(b'\n');
    }
    data
}

/// Returns kubeconfig path from `$HOME/.kube/config`.
fn default_kube_path() -> Option<PathBuf> {
    use dirs::home_dir;
    home_dir().map(|h| h.join(".kube").join("config"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::str::FromStr;

    #[test]
    fn kubeconfig_merge() {
        let kubeconfig1 = Kubeconfig {
            current_context: Some("default".into()),
            auth_infos: vec![NamedAuthInfo {
                name: "red-user".into(),
                auth_info: AuthInfo {
                    token: Some(SecretString::from_str("first-token").unwrap()),
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        let kubeconfig2 = Kubeconfig {
            current_context: Some("dev".into()),
            auth_infos: vec![
                NamedAuthInfo {
                    name: "red-user".into(),
                    auth_info: AuthInfo {
                        token: Some(SecretString::from_str("second-token").unwrap()),
                        username: Some("red-user".into()),
                        ..Default::default()
                    },
                },
                NamedAuthInfo {
                    name: "green-user".into(),
                    auth_info: AuthInfo {
                        token: Some(SecretString::from_str("new-token").unwrap()),
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        };

        let merged = kubeconfig1.merge(kubeconfig2).unwrap();
        // Preserves first `current_context`
        assert_eq!(merged.current_context, Some("default".into()));
        // Auth info with the same name does not overwrite
        assert_eq!(merged.auth_infos[0].name, "red-user".to_owned());
        assert_eq!(
            merged.auth_infos[0]
                .auth_info
                .token
                .as_ref()
                .map(|t| t.expose_secret().to_string()),
            Some("first-token".to_string())
        );
        // Even if it's not conflicting
        assert_eq!(merged.auth_infos[0].auth_info.username, None);
        // New named auth info is appended
        assert_eq!(merged.auth_infos[1].name, "green-user".to_owned());
    }

    #[test]
    fn kubeconfig_deserialize() {
        let config_yaml = "apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: LS0t<SNIP>LS0tLQo=
    server: https://ABCDEF0123456789.gr7.us-west-2.eks.amazonaws.com
  name: eks
- cluster:
    certificate-authority: /home/kevin/.minikube/ca.crt
    extensions:
    - extension:
        last-update: Thu, 18 Feb 2021 16:59:26 PST
        provider: minikube.sigs.k8s.io
        version: v1.17.1
      name: cluster_info
    server: https://192.168.49.2:8443
  name: minikube
contexts:
- context:
    cluster: minikube
    extensions:
    - extension:
        last-update: Thu, 18 Feb 2021 16:59:26 PST
        provider: minikube.sigs.k8s.io
        version: v1.17.1
      name: context_info
    namespace: default
    user: minikube
  name: minikube
- context:
    cluster: arn:aws:eks:us-west-2:012345678912:cluster/eks
    user: arn:aws:eks:us-west-2:012345678912:cluster/eks
  name: eks
current-context: minikube
kind: Config
preferences: {}
users:
- name: arn:aws:eks:us-west-2:012345678912:cluster/eks
  user:
    exec:
      apiVersion: client.authentication.k8s.io/v1alpha1
      args:
      - --region
      - us-west-2
      - eks
      - get-token
      - --cluster-name
      - eks
      command: aws
      env: null
      provideClusterInfo: false
- name: minikube
  user:
    client-certificate: /home/kevin/.minikube/profiles/minikube/client.crt
    client-key: /home/kevin/.minikube/profiles/minikube/client.key";

        let config = Kubeconfig::from_yaml(config_yaml).unwrap();

        assert_eq!(config.clusters[0].name, "eks");
        assert_eq!(config.clusters[1].name, "minikube");
        assert_eq!(
            config.clusters[1].cluster.extensions.as_ref().unwrap()[0]
                .extension
                .get("provider"),
            Some(&Value::String("minikube.sigs.k8s.io".to_owned()))
        );
    }

    #[test]
    fn kubeconfig_multi_document_merge() -> Result<(), KubeconfigError> {
        let config_yaml = r#"---
apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: aGVsbG8K
    server: https://0.0.0.0:6443
  name: k3d-promstack
contexts:
- context:
    cluster: k3d-promstack
    user: admin@k3d-promstack
  name: k3d-promstack
current-context: k3d-promstack
kind: Config
preferences: {}
users:
- name: admin@k3d-promstack
  user:
    client-certificate-data: aGVsbG8K
    client-key-data: aGVsbG8K
---
apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: aGVsbG8K
    server: https://0.0.0.0:6443
  name: k3d-k3s-default
contexts:
- context:
    cluster: k3d-k3s-default
    user: admin@k3d-k3s-default
  name: k3d-k3s-default
current-context: k3d-k3s-default
kind: Config
preferences: {}
users:
- name: admin@k3d-k3s-default
  user:
    client-certificate-data: aGVsbG8K
    client-key-data: aGVsbG8K
"#;
        let cfg = Kubeconfig::from_yaml(config_yaml)?;

        // Ensure we have data from both documents:
        assert_eq!(cfg.clusters[0].name, "k3d-promstack");
        assert_eq!(cfg.clusters[1].name, "k3d-k3s-default");

        Ok(())
    }

    #[test]
    fn kubeconfig_from_empty_string() {
        let cfg = Kubeconfig::from_yaml("").unwrap();

        assert_eq!(cfg, Kubeconfig::default());
    }

    #[test]
    fn authinfo_debug_does_not_output_password() {
        let authinfo_yaml = r#"
username: user
password: kube_rs
"#;
        let authinfo: AuthInfo = serde_yaml::from_str(authinfo_yaml).unwrap();
        let authinfo_debug_output = format!("{:?}", authinfo);
        let expected_output = "AuthInfo { \
        username: Some(\"user\"), \
        password: Some(Secret([REDACTED alloc::string::String])), \
        token: None, token_file: None, client_certificate: None, \
        client_certificate_data: None, client_key: None, \
        client_key_data: None, impersonate: None, \
        impersonate_groups: None, \
        auth_provider: None, \
        exec: None \
        }";

        assert_eq!(authinfo_debug_output, expected_output)
    }
}
