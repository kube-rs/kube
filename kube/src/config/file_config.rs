#![allow(missing_docs)]
use crate::{config::utils, error::ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// [`Kubeconfig`] represents information on how to connect to a remote Kubernetes cluster
///
/// Stored in `~/.kube/config` by default, but can be distributed across multiple paths in passed through `KUBECONFIG`.
/// An analogue of the [config type from client-go](https://github.com/kubernetes/kubernetes/blob/cea1d4e20b4a7886d8ff65f34c6d4f95efcb4742/staging/src/k8s.io/client-go/tools/clientcmd/api/types.go#L28-L55).
///
/// This type (and its children) are exposed primarily for convenience.
///
/// [`Config`][crate::Config] is the __intended__ developer interface to help create a [`Client`][crate::Client],
/// and this will handle the difference between in-cluster deployment and local development.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "apiVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// Preferences stores extensions for cli.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<NamedExtension>>,
}

/// NamedExtention associates name with extension.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NamedExtension {
    pub name: String,
    pub extension: serde_json::Value,
}

/// NamedCluster associates name with cluster.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NamedCluster {
    pub name: String,
    pub cluster: Cluster,
}

/// Cluster stores information to connect Kubernetes cluster.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Cluster {
    /// The address of the kubernetes cluster (https://hostname:port).
    pub server: String,
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
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NamedAuthInfo {
    pub name: String,
    #[serde(rename = "user")]
    pub auth_info: AuthInfo,
}

/// AuthInfo stores information to tell cluster who you are.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthInfo {
    /// The username for basic authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// The password for basic authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// The bearer token for authentication to the kubernetes cluster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key_data: Option<String>,

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

/// AuthProviderConfig stores auth for specified cloud provider.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthProviderConfig {
    pub name: String,
    pub config: HashMap<String, String>,
}

/// ExecConfig stores credential-plugin configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NamedContext {
    pub name: String,
    pub context: Context,
}

/// Context stores tuple of cluster and user information.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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
    pub fn read_from<P: AsRef<Path>>(path: P) -> Result<Kubeconfig> {
        let data = fs::read_to_string(&path).map_err(|source| ConfigError::ReadFile {
            path: path.as_ref().into(),
            source,
        })?;
        // support multiple documents
        let mut documents: Vec<Kubeconfig> = vec![];
        for doc in serde_yaml::Deserializer::from_str(&data) {
            let value = serde_yaml::Value::deserialize(doc).map_err(ConfigError::ParseYaml)?;
            let kconf = serde_yaml::from_value(value).map_err(ConfigError::ParseYaml)?;
            documents.push(kconf)
        }

        // Remap all files we read to absolute paths.
        let mut merged_docs = None;
        for mut config in documents {
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
        let config = merged_docs.ok_or_else(|| ConfigError::EmptyKubeconfig(path.as_ref().to_path_buf()))?;
        Ok(config)
    }

    /// Read a Config from `KUBECONFIG` or the the default location.
    pub fn read() -> Result<Kubeconfig> {
        match Self::from_env()? {
            Some(config) => Ok(config),
            None => {
                let path = utils::default_kube_path().ok_or(ConfigError::NoKubeconfigPath)?;
                Self::read_from(path)
            }
        }
    }

    /// Create `Kubeconfig` from `KUBECONFIG` environment variable.
    /// Supports list of files to be merged.
    ///
    /// # Panics
    ///
    /// Panics if `KUBECONFIG` value contains the NUL character.
    pub fn from_env() -> Result<Option<Self>> {
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
    fn merge(mut self, next: Kubeconfig) -> Result<Self> {
        if self.kind.is_some() && next.kind.is_some() && self.kind != next.kind {
            return Err(ConfigError::KindMismatch.into());
        }
        if self.api_version.is_some() && next.api_version.is_some() && self.api_version != next.api_version {
            return Err(ConfigError::ApiVersionMismatch.into());
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
    pub(crate) fn load_certificate_authority(&self) -> Result<Option<Vec<u8>>> {
        if self.certificate_authority.is_none() && self.certificate_authority_data.is_none() {
            return Ok(None);
        }
        let res =
            utils::data_or_file_with_base64(&self.certificate_authority_data, &self.certificate_authority)?;
        Ok(Some(res))
    }
}

impl AuthInfo {
    pub(crate) fn load_client_certificate(&self) -> Result<Vec<u8>> {
        utils::data_or_file_with_base64(&self.client_certificate_data, &self.client_certificate)
    }

    pub(crate) fn load_client_key(&self) -> Result<Vec<u8>> {
        utils::data_or_file_with_base64(&self.client_key_data, &self.client_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn kubeconfig_merge() {
        let kubeconfig1 = Kubeconfig {
            current_context: Some("default".into()),
            auth_infos: vec![NamedAuthInfo {
                name: "red-user".into(),
                auth_info: AuthInfo {
                    token: Some("first-token".into()),
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
                        token: Some("second-token".into()),
                        username: Some("red-user".into()),
                        ..Default::default()
                    },
                },
                NamedAuthInfo {
                    name: "green-user".into(),
                    auth_info: AuthInfo {
                        token: Some("new-token".into()),
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
        assert_eq!(merged.auth_infos[0].auth_info.token, Some("first-token".into()));
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

        let config: Kubeconfig = serde_yaml::from_str(config_yaml)
            .map_err(ConfigError::ParseYaml)
            .unwrap();

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
    fn kubeconfig_multi_document_merge() -> Result<()> {
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
        let file = tempfile::NamedTempFile::new().expect("create config tempfile");
        fs::write(file.path(), config_yaml).unwrap();
        let cfg = Kubeconfig::read_from(file.path())?;

        // Ensure we have data from both documents:
        assert_eq!(cfg.clusters[0].name, "k3d-promstack");
        assert_eq!(cfg.clusters[1].name, "k3d-k3s-default");

        Ok(())
    }
}
