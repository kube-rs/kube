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
use failure::ResultExt;
use crate::{Error, ErrorKind, Result};
use reqwest::{header, Certificate, Client, Identity};

use self::kube_config::KubeConfigLoader;

/// Configuration stores kubernetes path and client for requests.
#[derive(Clone)]
pub struct Configuration {
    pub base_path: String,
    pub client: Client,
}

impl Configuration {
    pub fn new(base_path: String, client: Client) -> Self {
        Configuration {
            base_path: base_path.to_owned(),
            client,
        }
    }
}

/// Returns a config includes authentication and cluster infomation from kubeconfig file.
///
/// # Example
/// ```no_run
/// use kube::config;
///
/// let kubeconfig = config::load_kube_config()
///     .expect("failed to load kubeconfig");
/// ```
pub fn load_kube_config() -> Result<Configuration> {
    load_kube_config_with(Default::default())
}

/// ConfigOptions stores options used when loading kubeconfig file.
#[derive(Default)]
pub struct ConfigOptions {
    pub context: Option<String>,
    pub cluster: Option<String>,
    pub user: Option<String>,
}

/// Returns a config includes authentication and cluster information from kubeconfig file.
///
/// # Example
/// ```no_run
/// use kube::config;
///
/// let kubeconfig = config::load_kube_config()
///     .expect("failed to load kubeconfig");
/// ```
pub fn load_kube_config_with(options: ConfigOptions) -> Result<Configuration> {
    let kubeconfig = utils::kubeconfig_path()
        .or_else(utils::default_kube_path)
        .ok_or_else(|| ErrorKind::KubeConfig("Unable to load file".into()))?;

    let loader =
        KubeConfigLoader::load(kubeconfig, options.context, options.cluster, options.user)?;
    let token = match &loader.user.token {
        Some(token) => Some(token.clone()),
        None => {
            if let Some(exec) = &loader.user.exec {
                let creds = exec::auth_exec(exec)?;
                let status = creds
                    .status
                    .ok_or_else(|| ErrorKind::KubeConfig("exec-plugin response did not contain a status".into()))?;
                status.token
            } else {
                None
            }
        }
    };

    let mut client_builder = Client::builder();

    if let Some(bundle) = loader.ca_bundle() {
        for ca in bundle? {
            let cert = Certificate::from_der(&ca.to_der().context(ErrorKind::SslError)?)
                .context(ErrorKind::SslError)?;
            client_builder = client_builder.add_root_certificate(cert);
        }
    }
    match loader.p12(" ") {
        Ok(p12) => {
            let req_p12 = Identity::from_pkcs12_der(&p12.to_der().context(ErrorKind::SslError)?, " ")
                .context(ErrorKind::SslError)?;
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
        (loader.user.username, loader.user.password),
    ) {
        (Ok(token), _) => {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", token))
                    .context(ErrorKind::KubeConfig("Invalid bearer token".to_string()))?,
            );
        }
        (_, (Some(u), Some(p))) => {
            let encoded = base64::encode(&format!("{}:{}", u, p));
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Basic {}", encoded))
                    .context(ErrorKind::KubeConfig("Invalid bearer token".to_string()))?,
            );
        }
        _ => {}
    }

    let client_builder = client_builder.default_headers(headers);

    Ok(Configuration::new(
        loader.cluster.server,
        client_builder.build()
            .context(ErrorKind::KubeConfig("Unable to build client".to_string()))?,
    ))
}

/// Returns a config which is used by clients within pods on kubernetes.
/// It will return an error if called from out of kubernetes cluster.
///
/// # Example
/// ```no_run
/// use kube::config;
///
/// let kubeconfig = config::incluster_config()
///     .expect("failed to load incluster config");
/// ```
pub fn incluster_config() -> Result<Configuration> {
    let server = incluster_config::kube_server().ok_or_else(||
        Error::from(ErrorKind::KubeConfig(format!(
            "Unable to load incluster config, {} and {} must be defined",
            incluster_config::SERVICE_HOSTENV,
            incluster_config::SERVICE_PORTENV
    ))))?;

    let ca = incluster_config::load_cert().context(ErrorKind::SslError)?;
    let req_ca = Certificate::from_der(&ca.to_der().context(ErrorKind::SslError)?)
        .context(ErrorKind::SslError)?;

    let token = incluster_config::load_token()
        .context(ErrorKind::KubeConfig("Unable to load in cluster token".to_string()))?;

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))
            .context(ErrorKind::KubeConfig("Invalid bearer token".to_string()))?,
    );

    let client_builder = Client::builder()
        .add_root_certificate(req_ca)
        .default_headers(headers);

    Ok(Configuration::new(
        server,
        client_builder.build()
            .context(ErrorKind::KubeConfig("Unable to build client".to_string()))?
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
