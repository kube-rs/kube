//! Kubernetes configuration objects from `~/.kube/config` or in cluster environment.
//!
//! Used to populate [`Config`] that is ultimately used to construct a [`Client`][crate::Client].
//!
//! Unless you have issues, prefer using [`Config::infer`] and pass it to a [`Client`][crate::Client].

mod file_config;
mod file_loader;
mod incluster_config;
#[cfg(any(feature = "native-tls", feature = "rustls-tls"))] mod tls;
mod utils;

use crate::{error::ConfigError, Result};
use file_loader::ConfigLoader;
pub use file_loader::KubeConfigOptions;
#[cfg(feature = "client")] pub(crate) use utils::read_file_to_string;

use http::header::HeaderMap;

use std::time::Duration;

/// Configuration object detailing things like cluster URL, default namespace, root certificates, and timeouts.
#[derive(Debug, Clone)]
pub struct Config {
    /// The configured cluster url
    pub cluster_url: http::Uri,
    /// The configured default namespace
    pub default_ns: String,
    /// The configured root certificate
    pub root_cert: Option<Vec<Vec<u8>>>,
    /// Default headers to be used to communicate with the Kubernetes API
    pub headers: HeaderMap,
    /// Timeout for calls to the Kubernetes API.
    ///
    /// A value of `None` means no timeout
    pub timeout: Option<std::time::Duration>,
    /// Whether to accept invalid ceritifacts
    pub accept_invalid_certs: bool,
    // TODO should keep client key and certificate separate. It's split later anyway.
    /// Client certificate and private key in PEM.
    identity_pem: Option<Vec<u8>>,
    /// Stores information to tell the cluster who you are.
    pub(crate) auth_info: AuthInfo,
}

impl Config {
    /// Construct a new config where only the `cluster_url` is set by the user.
    /// and everything else receives a default value.
    ///
    /// Most likely you want to use [`Config::infer`] to infer the config from
    /// the environment.
    pub fn new(cluster_url: http::Uri) -> Self {
        Self {
            cluster_url,
            default_ns: String::from("default"),
            root_cert: None,
            headers: HeaderMap::new(),
            timeout: Some(DEFAULT_TIMEOUT),
            accept_invalid_certs: false,
            identity_pem: None,
            auth_info: AuthInfo::default(),
        }
    }

    /// Infer the configuration from the environment
    ///
    /// Done by attempting to load in-cluster environment variables first, and
    /// then if that fails, trying the local kubeconfig.
    ///
    /// Fails if inference from both sources fails
    pub async fn infer() -> Result<Self> {
        match Self::from_cluster_env() {
            Err(cluster_env_err) => {
                tracing::trace!("No in-cluster config found: {}", cluster_env_err);
                tracing::trace!("Falling back to local kubeconfig");
                let config = Self::from_kubeconfig(&KubeConfigOptions::default())
                    .await
                    .map_err(|kubeconfig_err| ConfigError::ConfigInferenceExhausted {
                        cluster_env: Box::new(cluster_env_err),
                        kubeconfig: Box::new(kubeconfig_err),
                    })?;

                Ok(config)
            }
            success => success,
        }
    }

    /// Create configuration from the cluster's environment variables
    ///
    /// This follows the standard [API Access from a Pod](https://kubernetes.io/docs/tasks/access-application-cluster/access-cluster/#accessing-the-api-from-a-pod)
    /// and relies on you having the service account's token mounted,
    /// as well as having given the service account rbac access to do what you need.
    pub fn from_cluster_env() -> Result<Self> {
        let cluster_url = incluster_config::kube_server().ok_or(ConfigError::MissingInClusterVariables {
            hostenv: incluster_config::SERVICE_HOSTENV,
            portenv: incluster_config::SERVICE_PORTENV,
        })?;
        let cluster_url = cluster_url.parse::<http::Uri>()?;

        let default_ns = incluster_config::load_default_ns()
            .map_err(Box::new)
            .map_err(ConfigError::InvalidInClusterNamespace)?;

        let root_cert = incluster_config::load_cert()?;

        let token = incluster_config::load_token()
            .map_err(Box::new)
            .map_err(ConfigError::InvalidInClusterToken)?;

        Ok(Self {
            cluster_url,
            default_ns,
            root_cert: Some(root_cert),
            headers: HeaderMap::new(),
            timeout: Some(DEFAULT_TIMEOUT),
            accept_invalid_certs: false,
            identity_pem: None,
            auth_info: AuthInfo {
                token: Some(token),
                ..Default::default()
            },
        })
    }

    /// Create configuration from the default local config file
    ///
    /// This will respect the `$KUBECONFIG` evar, but otherwise default to `~/.kube/config`.
    /// You can also customize what context/cluster/user you want to use here,
    /// but it will default to the current-context.
    pub async fn from_kubeconfig(options: &KubeConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_options(options).await?;
        Self::new_from_loader(loader).await
    }

    /// Create configuration from a [`Kubeconfig`] struct
    ///
    /// This bypasses kube's normal config parsing to obtain custom functionality.
    /// Like if you need stacked kubeconfigs for instance - see #132
    pub async fn from_custom_kubeconfig(kubeconfig: Kubeconfig, options: &KubeConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_kubeconfig(kubeconfig, options).await?;
        Self::new_from_loader(loader).await
    }

    async fn new_from_loader(loader: ConfigLoader) -> Result<Self> {
        let cluster_url = loader.cluster.server.parse::<http::Uri>()?;

        let default_ns = loader
            .current_context
            .namespace
            .clone()
            .unwrap_or_else(|| String::from("default"));

        let mut accept_invalid_certs = false;
        let mut root_cert = None;
        let mut identity_pem = None;

        if let Some(ca_bundle) = loader.ca_bundle()? {
            for ca in &ca_bundle {
                accept_invalid_certs = hacky_cert_lifetime_for_macos(&ca);
            }
            root_cert = Some(ca_bundle);
        }

        // REVIEW Changed behavior. This no longer fails with invalid data in PEM.
        match loader.identity_pem() {
            Ok(id) => identity_pem = Some(id),
            Err(e) => {
                tracing::debug!("failed to load client identity from kubeconfig: {}", e);
                // last resort only if configs ask for it, and no client certs
                if let Some(true) = loader.cluster.insecure_skip_tls_verify {
                    accept_invalid_certs = true;
                }
            }
        }

        Ok(Self {
            cluster_url,
            default_ns,
            root_cert,
            headers: HeaderMap::new(),
            timeout: Some(DEFAULT_TIMEOUT),
            accept_invalid_certs,
            identity_pem,
            auth_info: loader.user,
        })
    }
}

// https://github.com/clux/kube-rs/issues/146#issuecomment-590924397
/// Default Timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(295);

// temporary catalina hack for openssl only
#[cfg(all(target_os = "macos", feature = "native-tls"))]
fn hacky_cert_lifetime_for_macos(ca: &[u8]) -> bool {
    use openssl::x509::X509;
    let ca = X509::from_der(ca).expect("valid der is a der");
    ca.not_before()
        .diff(ca.not_after())
        .map(|d| d.days.abs() > 824)
        .unwrap_or(false)
}

#[cfg(any(not(target_os = "macos"), not(feature = "native-tls")))]
fn hacky_cert_lifetime_for_macos(_: &[u8]) -> bool {
    false
}

// Expose raw config structs
pub use file_config::{
    AuthInfo, AuthProviderConfig, Cluster, Context, ExecConfig, Kubeconfig, NamedAuthInfo, NamedCluster,
    NamedContext, NamedExtension, Preferences,
};


#[cfg(test)]
mod tests {
    #[cfg(not(feature = "client"))] // want to ensure this works without client features
    #[tokio::test]
    async fn config_loading_on_small_feature_set() {
        use super::Config;
        let cfgraw = r#"
        apiVersion: v1
        clusters:
        - cluster:
            certificate-authority-data: aGVsbG8K
            server: https://0.0.0.0:6443
          name: k3d-test
        contexts:
        - context:
            cluster: k3d-test
            user: admin@k3d-test
          name: k3d-test
        current-context: k3d-test
        kind: Config
        preferences: {}
        users:
        - name: admin@k3d-test
          user:
            client-certificate-data: aGVsbG8K
            client-key-data: aGVsbG8K
        "#;
        let file = tempfile::NamedTempFile::new().expect("create config tempfile");
        std::fs::write(file.path(), cfgraw).unwrap();
        std::env::set_var("KUBECONFIG", file.path());
        let kubeconfig = Config::infer().await.unwrap();
        assert_eq!(kubeconfig.cluster_url, "https://0.0.0.0:6443/");
    }
}
