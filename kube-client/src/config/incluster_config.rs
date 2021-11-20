use std::env;

use thiserror::Error;

// Old method to connect to kubernetes
const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";
// New method to connect to kubernetes
const SERVICE_DNS: &str = "kubernetes.default.svc";
// Mounted credential files
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";
const SERVICE_DEFAULT_NS: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

/// Errors from loading in-cluster config
#[derive(Error, Debug)]
pub enum Error {
    /// Required envionment variables were not set
    #[error(
        "missing environment variables {} and/or {}",
        SERVICE_HOSTENV,
        SERVICE_PORTENV
    )]
    MissingEnvironmentVariables,

    /// Failed to read the default namespace for the service account
    #[error("failed to read the default namespace: {0}")]
    ReadDefaultNamespace(#[source] std::io::Error),

    /// Failed to read the token for the service account
    #[error("failed to read the SA token: {0}")]
    ReadToken(#[source] std::io::Error),

    /// Failed to read a certificate bundle
    #[error("failed to read a certificate bundle: {0}")]
    ReadCertificateBundle(#[source] std::io::Error),

    /// Failed to parse cluster url
    #[error("failed to parse cluster url: {0}")]
    ParseClusterUrl(#[source] http::uri::InvalidUri),

    /// Failed to parse PEM-encoded certificates
    #[error("failed to parse PEM-encoded certificates: {0}")]
    ParseCertificates(#[source] pem::PemError),
}

/// Returns Kubernetes address from specified environment variables.
pub fn kube_server() -> Result<http::Uri, Error> {
    kube_host_port()
        .ok_or(Error::MissingEnvironmentVariables)?
        .parse::<http::Uri>()
        .map_err(Error::ParseClusterUrl)
}

pub fn kube_dns() -> http::Uri {
    http::Uri::builder()
        .scheme("https")
        .authority(SERVICE_DNS)
        .path_and_query("/")
        .build()
        .unwrap()
}

fn kube_host_port() -> Option<String> {
    let host = kube_host()?;
    let port = kube_port()?;
    Some(format!("https://{}:{}", host, port))
}

fn kube_host() -> Option<String> {
    env::var(SERVICE_HOSTENV).ok()
}

fn kube_port() -> Option<String> {
    env::var(SERVICE_PORTENV).ok()
}

/// Returns token from specified path in cluster.
pub fn load_token() -> Result<String, Error> {
    std::fs::read_to_string(&SERVICE_TOKENFILE).map_err(Error::ReadToken)
}

/// Returns certification from specified path in cluster.
pub fn load_cert() -> Result<Vec<Vec<u8>>, Error> {
    let certs = std::fs::read(&SERVICE_CERTFILE).map_err(Error::ReadCertificateBundle)?;
    super::certs(&certs).map_err(Error::ParseCertificates)
}

/// Returns the default namespace from specified path in cluster.
pub fn load_default_ns() -> Result<String, Error> {
    std::fs::read_to_string(&SERVICE_DEFAULT_NS).map_err(Error::ReadDefaultNamespace)
}

#[test]
fn test_kube_host() {
    let expected = "fake.io";
    env::set_var(SERVICE_HOSTENV, expected);
    assert_eq!(kube_host().unwrap(), expected);
    kube_dns(); // verify kube_dns always unwraps
}

#[test]
fn test_kube_port() {
    let expected = "8080";
    env::set_var(SERVICE_PORTENV, expected);
    assert_eq!(kube_port().unwrap(), expected);
}

#[test]
fn test_kube_server() {
    let host = "fake.io";
    let port = "8080";
    env::set_var(SERVICE_HOSTENV, host);
    env::set_var(SERVICE_PORTENV, port);
    assert_eq!(kube_server().unwrap(), "https://fake.io:8080");
}
