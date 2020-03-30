//! In cluster or out of cluster kubeconfig to be used by an api client
//!
//! You primarily want to interact with `Configuration`,
//! and its associated load functions.
//!
//! The full `Config` and child-objects are exposed here for convenience only.

mod apis;
pub mod incluster_config;
pub(crate) mod kube_config;
pub(crate) mod utils;

use crate::{Error, Result};

pub use self::kube_config::ConfigLoader;

/// Configuration stores kubernetes path and client for requests.
#[derive(Clone, Debug)]
pub struct Configuration {
    pub base_path: String,
    /// The current default namespace. This will be "default" while running outside of a cluster,
    /// and will be the namespace of the pod while running inside a cluster.
    pub default_ns: String,
}

impl Configuration {
    pub fn new(base_path: String) -> Self {
        Self::with_default_ns(base_path, "default".to_string())
    }

    /// Returns a config which includes authentication and cluster information from kubeconfig file.
    pub async fn new_from_options(options: &ConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_options(options).await?;
        Ok(Self::new(loader.cluster.server))
    }

    pub fn with_default_ns(base_path: String, default_ns: String) -> Self {
        Self {
            base_path,
            default_ns,
        }
    }

    /// Infer the config type and return it
    ///
    /// Done by attempting to load in-cluster evars first,
    /// then if that fails, try the full local kube config.
    pub async fn infer() -> Result<Self> {
        let cfg = match incluster_config() {
            Err(e) => {
                trace!("No in-cluster config found: {}", e);
                trace!("Falling back to local kube config");
                load_kube_config().await?
            }
            Ok(o) => o,
        };
        Ok(cfg)
    }
}

/// Returns a config includes authentication and cluster infomation from kubeconfig file.
pub async fn load_kube_config() -> Result<Configuration> {
    Configuration::new_from_options(&ConfigOptions::default()).await
}

/// ConfigOptions stores options used when loading kubeconfig file.
#[derive(Default, Clone)]
pub struct ConfigOptions {
    pub context: Option<String>,
    pub cluster: Option<String>,
    pub user: Option<String>,
}

/// Returns a config which is used by clients within pods on kubernetes.
///
/// It will return an error if called from out of kubernetes cluster.
pub fn incluster_config() -> Result<Configuration> {
    let server = incluster_config::kube_server().ok_or_else(|| {
        Error::KubeConfig(format!(
            "Unable to load incluster config, {} and {} must be defined",
            incluster_config::SERVICE_HOSTENV,
            incluster_config::SERVICE_PORTENV
        ))
    })?;

    let default_ns = incluster_config::load_default_ns()
        .map_err(|e| Error::KubeConfig(format!("Unable to load incluster default namespace: {}", e)))?;

    Ok(Configuration::with_default_ns(server, default_ns))
}

pub(crate) fn incluster_client() -> Result<reqwest::ClientBuilder> {
    let cert = incluster_config::load_cert()?;

    let token = incluster_config::load_token()
        .map_err(|e| Error::KubeConfig(format!("Unable to load in cluster token: {}", e)))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
            .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
    );

    Ok(reqwest::Client::builder()
        .add_root_certificate(cert)
        .default_headers(headers))
}

// Expose raw config structs
pub use apis::{
    AuthInfo, AuthProviderConfig, Cluster, Config, Context, ExecConfig, NamedCluster, NamedContext,
    NamedExtension, Preferences,
};
