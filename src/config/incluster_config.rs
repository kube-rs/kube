use std::env;

use failure::Error;
use openssl::x509::X509;

use crate::config::utils;

pub const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
pub const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";

/// Returns kubernetes address from specified environment variables.
pub fn kube_server() -> Option<String> {
    let f = |(h, p)| format!("https://{}:{}", h, p);
    kube_host().and_then(|h| kube_port().map(|p| f((h, p))))
}

fn kube_host() -> Option<String> {
    env::var(SERVICE_HOSTENV).ok()
}

fn kube_port() -> Option<String> {
    env::var(SERVICE_PORTENV).ok()
}

/// Returns token from specified path in cluster.
pub fn load_token() -> Result<String, Error> {
    utils::data_or_file(&None, &Some(SERVICE_TOKENFILE.to_string()))
}

/// Returns certification from specified path in cluster.
pub fn load_cert() -> Result<X509, Error> {
    let ca = utils::data_or_file_with_base64(&None, &Some(SERVICE_CERTFILE.to_string()))?;
    X509::from_pem(&ca).map_err(Error::from)
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
