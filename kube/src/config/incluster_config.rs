use std::env;

use crate::{Error, Result};
use reqwest::Certificate;

use crate::{config::utils, error::ConfigError};

pub const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
pub const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";
const SERVICE_DEFAULT_NS: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

/// Returns Kubernetes address from specified environment variables.
pub fn kube_server() -> Option<String> {
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
pub fn load_token() -> Result<String> {
    utils::data_or_file(&None, &Some(SERVICE_TOKENFILE))
}

/// Returns certification from specified path in cluster.
pub fn load_cert() -> Result<Vec<Certificate>> {
    let ca = utils::data_or_file_with_base64(&None, &Some(SERVICE_CERTFILE))?;
    let pems = pem::parse_many(ca);

    pems.into_iter()
        .map(|pem| {
            Certificate::from_pem(&pem::encode(&pem).into_bytes())
                .map_err(ConfigError::LoadCert)
                .map_err(Error::from)
        })
        .collect::<Result<Vec<_>>>()
}

/// Returns the default namespace from specified path in cluster.
pub fn load_default_ns() -> Result<String> {
    utils::data_or_file(&None, &Some(SERVICE_DEFAULT_NS))
}

#[test]
fn test_kube_host() {
    let expected = "fake.io";
    env::set_var(SERVICE_HOSTENV, expected);
    assert_eq!(kube_host().unwrap(), expected);
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
