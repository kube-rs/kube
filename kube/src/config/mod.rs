//! Kubernetes configuration objects from `~/.kube/config` or in cluster environment.
//!
//! Used to populate [`Config`] that is ultimately used to construct a [`Client`][crate::Client].
//!
//! Unless you have issues, prefer using [`Config::infer`] and pass it to a [`Client`][crate::Client].

mod exec;
mod file_config;
mod file_loader;
mod incluster_config;
mod utils;

use crate::{error::ConfigError, Error, Result};
use file_loader::ConfigLoader;
pub use file_loader::KubeConfigOptions;

use chrono::{DateTime, Utc};
use reqwest::{
    header::{self, HeaderMap},
    Certificate,
};
use tokio::sync::Mutex;

use std::{sync::Arc, time::Duration};

/// Regardless of tls type, a Certificate Der is always a byte array
#[derive(Debug, Clone)]
pub struct Der(pub Vec<u8>);

use std::convert::TryFrom;
impl TryFrom<Der> for Certificate {
    type Error = Error;

    fn try_from(val: Der) -> Result<Certificate> {
        Certificate::from_der(&val.0)
            .map_err(ConfigError::LoadCert)
            .map_err(Error::from)
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Authentication {
    None,
    Basic(String),
    Token(String),
    RefreshableToken(Arc<Mutex<(String, DateTime<Utc>)>>, ConfigLoader),
}

impl Authentication {
    async fn to_header(&self) -> Result<Option<header::HeaderValue>, ConfigError> {
        match self {
            Self::None => Ok(None),
            Self::Basic(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBasicAuth)?,
            )),
            Self::Token(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBearerToken)?,
            )),
            Self::RefreshableToken(data, loader) => {
                let mut locked_data = data.lock().await;
                // Add some wiggle room onto the current timestamp so we don't get any race
                // conditions where the token expires while we are refreshing
                if chrono::Utc::now() + chrono::Duration::seconds(60) >= locked_data.1 {
                    if let Authentication::RefreshableToken(d, _) = load_auth_header(loader)? {
                        let (new_token, new_expire) = Arc::try_unwrap(d)
                            .expect("Unable to unwrap Arc, this is likely a programming error")
                            .into_inner();
                        locked_data.0 = new_token;
                        locked_data.1 = new_expire;
                    } else {
                        return Err(ConfigError::UnrefreshableTokenResponse);
                    }
                }
                Ok(Some(
                    header::HeaderValue::from_str(&locked_data.0).map_err(ConfigError::InvalidBearerToken)?,
                ))
            }
        }
    }
}

/// Configuration object detailing things like cluster URL, default namespace, root certificates, and timeouts.
#[derive(Debug, Clone)]
pub struct Config {
    /// The configured cluster url
    pub cluster_url: reqwest::Url,
    /// The configured default namespace
    pub default_ns: String,
    /// The configured root certificate
    pub root_cert: Option<Vec<Der>>,
    /// Default headers to be used to communicate with the Kubernetes API
    pub headers: HeaderMap,
    /// Timeout for calls to the Kubernetes API.
    ///
    /// A value of `None` means no timeout
    pub timeout: Option<std::time::Duration>,
    /// Whether to accept invalid ceritifacts
    pub accept_invalid_certs: bool,
    /// Proxy to send requests to Kubernetes API through
    pub(crate) proxy: Option<reqwest::Proxy>,
    /// The identity to use for communicating with the Kubernetes API
    /// along wit the password to decrypt it.
    ///
    /// This is stored in a raw buffer form so that Config can implement `Clone`
    /// (since [`reqwest::Identity`] does not currently implement `Clone`)
    pub(crate) identity: Option<(Vec<u8>, String)>,
    /// The authentication header from the credentials available in the kubeconfig. This supports
    /// exec plugins as well as specified in
    /// <https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins>
    pub(crate) auth_header: Authentication,
}

impl Config {
    /// Construct a new config where only the `cluster_url` is set by the user.
    /// and everything else receives a default value.
    ///
    /// Most likely you want to use [`Config::infer`] to infer the config from
    /// the environment.
    pub fn new(cluster_url: reqwest::Url) -> Self {
        Self {
            cluster_url,
            default_ns: String::from("default"),
            root_cert: None,
            headers: HeaderMap::new(),
            timeout: Some(DEFAULT_TIMEOUT),
            accept_invalid_certs: false,
            proxy: None,
            identity: None,
            auth_header: Authentication::None,
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
                trace!("No in-cluster config found: {}", cluster_env_err);
                trace!("Falling back to local kubeconfig");
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
        let cluster_url = reqwest::Url::parse(&cluster_url)?;

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
            proxy: None,
            identity: None,
            auth_header: Authentication::Token(format!("Bearer {}", token)),
        })
    }

    /// Create configuration from the default local config file
    ///
    /// This will respect the `$KUBECONFIG` evar, but otherwise default to `~/.kube/config`.
    /// You can also customize what context/cluster/user you want to use here,
    /// but it will default to the current-context.
    pub async fn from_kubeconfig(options: &KubeConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_options(options).await?;
        Self::new_from_loader(loader)
    }

    /// Create configuration from a [`Kubeconfig`] struct
    ///
    /// This bypasses kube's normal config parsing to obtain custom functionality.
    /// Like if you need stacked kubeconfigs for instance - see #132
    pub async fn from_custom_kubeconfig(kubeconfig: Kubeconfig, options: &KubeConfigOptions) -> Result<Self> {
        let loader = ConfigLoader::new_from_kubeconfig(kubeconfig, options).await?;
        Self::new_from_loader(loader)
    }

    fn new_from_loader(loader: ConfigLoader) -> Result<Self> {
        let cluster_url = reqwest::Url::parse(&loader.cluster.server)?;

        let default_ns = loader
            .current_context
            .namespace
            .clone()
            .unwrap_or_else(|| String::from("default"));

        let mut accept_invalid_certs = false;
        let mut root_cert = None;
        let mut identity = None;

        if let Some(ca_bundle) = loader.ca_bundle()? {
            for ca in &ca_bundle {
                accept_invalid_certs = hacky_cert_lifetime_for_macos(&ca);
            }
            root_cert = Some(ca_bundle);
        }

        match loader.identity(IDENTITY_PASSWORD) {
            Ok(id) => identity = Some(id),
            Err(e) => {
                debug!("failed to load client identity from kubeconfig: {}", e);
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
            proxy: None,
            identity: identity.map(|i| (i, String::from(IDENTITY_PASSWORD))),
            auth_header: load_auth_header(&loader)?,
        })
    }

    /// Get a valid HTTP `Authorization` header that can authenticate to the cluster
    ///
    /// Will renew tokens if required (and configured to).
    ///
    /// NOTE: This is `None` if the `Config` isn't configured to use token-based authentication
    /// (such as anonymous access, or certificate-based authentication).
    pub async fn get_auth_header(&self) -> Result<Option<header::HeaderValue>, ConfigError> {
        self.auth_header.to_header().await
    }

    // The identity functions are used to parse the stored identity buffer
    // into an `reqwest::Identity` type. We do this because `reqwest::Identity`
    // is not `Clone`. This allows us to store and clone the buffer and supply
    // the `Identity` in a just-in-time fashion.
    //
    // Note: this should be removed if/when reqwest implements [`Clone` for
    // `Identity`](https://github.com/seanmonstar/reqwest/issues/871)

    // feature = "rustls-tls" assumes the buffer is pem
    #[cfg(feature = "rustls-tls")]
    pub(crate) fn identity(&self) -> Option<reqwest::Identity> {
        let (identity, _identity_password) = self.identity.as_ref()?;
        Some(reqwest::Identity::from_pem(identity).expect("Identity buffer was not valid identity"))
    }

    // feature = "native-tls" assumes the buffer is pkcs12 der
    #[cfg(feature = "native-tls")]
    pub(crate) fn identity(&self) -> Option<reqwest::Identity> {
        let (identity, identity_password) = self.identity.as_ref()?;
        Some(
            reqwest::Identity::from_pkcs12_der(identity, identity_password)
                .expect("Identity buffer was not valid identity"),
        )
    }

    /// Configure a proxy for this kube config
    ///
    /// ```no_run
    /// use kube::{Config, config};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let mut config = Config::from_kubeconfig(&config::KubeConfigOptions::default()).await?;
    ///     let proxy = reqwest::Proxy::http("https://localhost:8080")?;
    ///     let config = config.proxy(proxy);
    ///     Ok(())
    /// }
    /// ```
    pub fn proxy(mut self, proxy: reqwest::Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }
}

/// Loads the authentication header from the credentials available in the kubeconfig. This supports
/// exec plugins as well as specified in
/// https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins
fn load_auth_header(loader: &ConfigLoader) -> Result<Authentication, ConfigError> {
    let (raw_token, expiration) = match &loader.user.token {
        Some(token) => (Some(token.clone()), None),
        None => {
            if let Some(exec) = &loader.user.exec {
                let creds = exec::auth_exec(exec)?;
                let status = creds.status.ok_or(ConfigError::ExecPluginFailed)?;
                let expiration = match status.expiration_timestamp {
                    Some(ts) => Some(
                        ts.parse::<DateTime<Utc>>()
                            .map_err(ConfigError::MalformedTokenExpirationDate)?,
                    ),
                    None => None,
                };
                (status.token, expiration)
            } else {
                (None, None)
            }
        }
    };
    match (
        utils::data_or_file(&raw_token, &loader.user.token_file),
        (&loader.user.username, &loader.user.password),
        expiration,
    ) {
        (Ok(token), _, None) => Ok(Authentication::Token(format!("Bearer {}", token))),
        (Ok(token), _, Some(expire)) => Ok(Authentication::RefreshableToken(
            Arc::new(Mutex::new((format!("Bearer {}", token), expire))),
            loader.clone(),
        )),
        (_, (Some(u), Some(p)), _) => {
            let encoded = base64::encode(&format!("{}:{}", u, p));
            Ok(Authentication::Basic(format!("Basic {}", encoded)))
        }
        _ => Ok(Authentication::None),
    }
}

// https://github.com/clux/kube-rs/issues/146#issuecomment-590924397
/// Default Timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(295);
const IDENTITY_PASSWORD: &str = " ";

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
    AuthInfo, AuthProviderConfig, Cluster, Context, ExecConfig, Kubeconfig, NamedAuthInfo, NamedCluster,
    NamedContext, NamedExtension, Preferences,
};
