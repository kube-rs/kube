//! In cluster or out of cluster kubeconfig to be used by an api client
//!
//! You primarily want to interact with `Configuration`,
//! and its associated load functions.
//!
//! The full `Config` and child-objects are exposed here for convenience only.

mod apis;
mod exec;
mod incluster_config;
mod kube_config;
mod utils;

use base64;
use crate::{Error, Result};
use reqwest::{header, Certificate, Client, ClientBuilder, Identity};

use self::kube_config::KubeConfigLoader;

/// Configuration stores kubernetes path and client for requests.
#[derive(Clone)]
pub struct Configuration {
    pub base_path: String,
    pub client: Client,

    /// The current default namespace. This will be "default" while running outside of a cluster,
    /// and will be the namespace of the pod while running inside a cluster.
    pub default_ns: String,
}

impl Configuration {
    pub fn new(base_path: String, client: Client) -> Self {
        Self::with_default_ns(base_path, client, "default".to_string())
    }

    pub fn with_default_ns(base_path: String, client: Client, default_ns: String) -> Self {
        Configuration {
            base_path: base_path.to_owned(),
            client,
            default_ns,
        }
    }
}

/// Returns a config includes authentication and cluster infomation from kubeconfig file.
pub async fn load_kube_config() -> Result<Configuration> {
    load_kube_config_with(Default::default()).await
}

/// ConfigOptions stores options used when loading kubeconfig file.
#[derive(Default)]
pub struct ConfigOptions {
    pub context: Option<String>,
    pub cluster: Option<String>,
    pub user: Option<String>,
}

/// Returns a config which includes authentication and cluster information from kubeconfig file.
pub async fn load_kube_config_with(options: ConfigOptions) -> Result<Configuration> {
    let result = create_client_builder(options).await?;
    Ok(Configuration::new(
        result.1.cluster.server,
        result.0.build()
            .map_err(|e| Error::KubeConfig(format!("Unable to build client: {}", e)))?,
    ))
}

/// Returns a client builder and config loader, based on the cluster information from the kubeconfig file.
///
/// This allows to create your custom reqwest client for using with the cluster API.
pub async fn create_client_builder(options: ConfigOptions) -> Result<(ClientBuilder,KubeConfigLoader)> {
    let kubeconfig = utils::find_kubeconfig()
        .map_err(|e| Error::KubeConfig(format!("Unable to load file: {}", e)))?;

    let loader =
        KubeConfigLoader::load(kubeconfig, options.context, options.cluster, options.user).await?;

    let token = match &loader.user.token {
        Some(token) => Some(token.clone()),
        None => {
            if let Some(exec) = &loader.user.exec {
                let creds = exec::auth_exec(exec)?;
                let status = creds
                    .status
                    .ok_or_else(|| Error::KubeConfig("exec-plugin response did not contain a status".into()))?;
                status.token
            } else {
                None
            }
        }
    };

    let mut client_builder = Client::builder();

    if let Some(bundle) = loader.ca_bundle() {
        for ca in bundle? {
            let cert = Certificate::from_der(&ca.to_der().map_err(|e| Error::SslError(format!("{}", e)))?)
                .map_err(Error::ReqwestError)?;
            client_builder = client_builder.add_root_certificate(cert);
        }
    }
    match loader.p12(" ") {
        Ok(p12) => {
            let der = p12.to_der().map_err(|e| Error::SslError(format!("{}", e)))?;
            let req_p12 = Identity::from_pkcs12_der(&der, " ")
                .map_err(Error::ReqwestError)?;
            client_builder = client_builder.identity(req_p12);
        }
        Err(_) => {
            // last resort only if configs ask for it, and no client certs
            if let Some(true) = loader.cluster.insecure_skip_tls_verify {
                client_builder = client_builder.danger_accept_invalid_certs(true);
            }
        }
    }

    let mut headers = header::HeaderMap::new();

    match (
        utils::data_or_file(&token, &loader.user.token_file),
        (&loader.user.username, &loader.user.password),
    ) {
        (Ok(token), _) => {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", token))
                    .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
            );
        }
        (_, (Some(u), Some(p))) => {
            let encoded = base64::encode(&format!("{}:{}", u, p));
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Basic {}", encoded))
                    .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
            );
        }
        _ => {}
    }

    Ok((client_builder.default_headers(headers), loader))

}

/// Returns a config which is used by clients within pods on kubernetes.
///
/// It will return an error if called from out of kubernetes cluster.
pub fn incluster_config() -> Result<Configuration> {
    let server = incluster_config::kube_server().ok_or_else(||
        Error::KubeConfig(format!(
            "Unable to load incluster config, {} and {} must be defined",
            incluster_config::SERVICE_HOSTENV,
            incluster_config::SERVICE_PORTENV
    )))?;

    let ca = incluster_config::load_cert()
        .map_err(|e| Error::SslError(format!("{}", e)))?;
    let der = ca.to_der().map_err(|e| Error::SslError(format!("{}", e)))?;
    let req_ca = Certificate::from_der(&der)
        .map_err(|e| Error::SslError(format!("{}", e)))?;

    let token = incluster_config::load_token()
        .map_err(|e| Error::KubeConfig(format!("Unable to load in cluster token: {}", e)))?;

    let default_ns = incluster_config::load_default_ns().map_err(|e| Error::KubeConfig(
        format!("Unable to load incluster default namespace: {}", e),
    ))?;

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
    );

    let client_builder = Client::builder()
        .add_root_certificate(req_ca)
        .default_headers(headers);

    Ok(Configuration::with_default_ns(
        server,
        client_builder.build()
            .map_err(|e| Error::KubeConfig(format!("Unable to build client: {}", e)))?,
        default_ns,
    ))
}


// Expose raw config structs
pub use apis::{
    Config,
    Preferences,
    NamedExtension,
    NamedCluster,
    Cluster,
    AuthInfo,
    AuthProviderConfig,
    ExecConfig,
    NamedContext,
    Context,
};
