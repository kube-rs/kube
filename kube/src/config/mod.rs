//! In cluster or out of cluster kubeconfig to be used by an api client
//!
//! You primarily want to interact with `Configuration`,
//! and its associated load functions.
//!
//! The full `Config` and child-objects are exposed here for convenience only.

mod exec;
mod file_config;
mod file_loader;
mod incluster_config;
mod utils;

use crate::{Error, Result};
use file_loader::{ConfigLoader, Der, KubeConfigOptions};

/// Configuration object detailing things like cluster_url, default namespace, root certificates, and timeouts
#[derive(Debug)]
pub struct Config {
    /// The configured cluster url
    pub cluster_url: reqwest::Url,
    /// The configured default namespace
    pub default_ns: String,
    /// The configured root certificate
    pub root_cert: Option<reqwest::Certificate>,
    /// Default headers to be used to communicate with the kubernetes API
    pub headers: reqwest::header::HeaderMap,
    /// Timeout for calls to the kubernetes API.
    ///
    /// A value of `None` means no timeout
    pub(crate) timeout: Option<std::time::Duration>,
    /// Whether to accept invalid ceritifacts
    pub(crate) accept_invalid_certs: bool,
    /// The identity to use for communicating with the kubernetes API
    pub(crate) identity: Option<reqwest::Identity>,
}

impl Config {
    /// Infer the config from the environment
    ///
    /// Done by attempting to load in-cluster environment variables first, and
    /// then if that fails, trying the local kube config.
    ///
    /// Fails if inference from both sources fails
    pub async fn infer() -> Result<Self> {
        match Self::new_from_cluster_env() {
            Err(e1) => {
                trace!("No in-cluster config found: {}", e1);
                trace!("Falling back to local kube config");
                let config = Self::new_from_kube_config(&KubeConfigOptions::default())
                    .await
                    .map_err(|e2| Error::KubeConfig(format!("Failed to infer config: {}, {}", e1, e2)))?;

                Ok(config)
            }
            success => success,
        }
    }

    /// Read the config from the cluster's environment variables
    pub fn new_from_cluster_env() -> Result<Self> {
        let cluster_url = incluster_config::kube_server().ok_or_else(|| {
            Error::KubeConfig(format!(
                "Unable to load in cluster config, {} and {} must be defined",
                incluster_config::SERVICE_HOSTENV,
                incluster_config::SERVICE_PORTENV
            ))
        })?;
        let cluster_url = reqwest::Url::parse(&cluster_url)
            .map_err(|e| Error::KubeConfig(format!("Malformed url: {}", e)))?;

        let default_ns = incluster_config::load_default_ns()
            .map_err(|e| Error::KubeConfig(format!("Unable to load incluster default namespace: {}", e)))?;

        let root_cert = incluster_config::load_cert()?;

        let token = incluster_config::load_token()
            .map_err(|e| Error::KubeConfig(format!("Unable to load in cluster token: {}", e)))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
        );

        Ok(Self {
            cluster_url,
            default_ns,
            root_cert: Some(root_cert),
            headers,
            timeout: None,
            accept_invalid_certs: false,
            identity: None,
        })
    }

    /// Returns a client builder based on the cluster information from the kubeconfig file.
    ///
    /// This allows to create your custom reqwest client for using with the cluster API.
    pub async fn new_from_kube_config(options: &KubeConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_options(options).await?;
        let cluster_url = reqwest::Url::parse(&loader.cluster.server)
            .map_err(|e| Error::KubeConfig(format!("Malformed url: {}", e)))?;

        let default_ns = loader
            .current_context
            .namespace
            .clone()
            .unwrap_or_else(|| String::from("default"));

        let token = match &loader.user.token {
            Some(token) => Some(token.clone()),
            None => {
                if let Some(exec) = &loader.user.exec {
                    let creds = exec::auth_exec(exec)?;
                    let status = creds.status.ok_or_else(|| {
                        Error::KubeConfig("exec-plugin response did not contain a status".into())
                    })?;
                    status.token
                } else {
                    None
                }
            }
        };

        let timeout = std::time::Duration::new(295, 0);
        let mut accept_invalid_certs = false;
        let mut root_cert = None;
        let mut identity = None;

        if let Some(ca_bundle) = loader.ca_bundle()? {
            use std::convert::TryInto;
            for ca in ca_bundle {
                accept_invalid_certs = hacky_cert_lifetime_for_macos(&ca);
                root_cert = Some(ca.try_into()?);
            }
        }

        match loader.identity(" ") {
            Ok(id) => identity = Some(id),
            Err(e) => {
                debug!("failed to load client identity from kube config: {}", e);
                // last resort only if configs ask for it, and no client certs
                if let Some(true) = loader.cluster.insecure_skip_tls_verify {
                    accept_invalid_certs = true;
                }
            }
        }

        let mut headers = reqwest::header::HeaderMap::new();

        match (
            utils::data_or_file(&token, &loader.user.token_file),
            (&loader.user.username, &loader.user.password),
        ) {
            (Ok(token), _) => {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                        .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
                );
            }
            (_, (Some(u), Some(p))) => {
                let encoded = base64::encode(&format!("{}:{}", u, p));
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded))
                        .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
                );
            }
            _ => {}
        }

        Ok(Self {
            cluster_url,
            default_ns,
            root_cert,
            headers,
            timeout: Some(timeout),
            accept_invalid_certs,
            identity,
        })
    }
}

// temporary catalina hack for openssl only
#[cfg(all(target_os = "macos", feature = "native-tls"))]
fn hacky_cert_lifetime_for_macos(ca: &Der) -> bool {
    use openssl::x509::X509;
    let ca = X509::from_der(&ca.0).expect("valid der is a der");
    ca.not_before()
        .diff(ca.not_after())
        .map(|d| d.days.abs() > 824)
        .unwrap_or(false)
}

#[cfg(any(not(target_os = "macos"), not(feature = "native-tls")))]
fn hacky_cert_lifetime_for_macos(_: &Der) -> bool {
    false
}

// Expose raw config structs
pub use file_config::{
    AuthInfo, AuthProviderConfig, Cluster, Context, ExecConfig, KubeConfig, NamedCluster, NamedContext,
    NamedExtension, Preferences,
};
