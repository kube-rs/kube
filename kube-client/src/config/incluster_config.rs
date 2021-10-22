use std::env;

use crate::{config::utils, Result};

// Old method to connect to kubernetes
pub const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
pub const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";
// New method to connect to kubernetes
pub const SERVICE_DNS: &str = "kubernetes.default.svc";
// Mounted credential files
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";
const SERVICE_DEFAULT_NS: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

/// Returns Kubernetes address from specified environment variables.
pub fn kube_server() -> Option<String> {
    let host = kube_host()?;
    let port = kube_port()?;
    Some(format!("https://{}:{}", host, port))
}

pub fn kube_dns() -> http::Uri {
    http::Uri::builder()
        .scheme("https")
        .authority(SERVICE_DNS)
        .path_and_query("/")
        .build()
        .unwrap()
}

fn kube_host() -> Option<String> {
    env::var(SERVICE_HOSTENV).ok()
}

fn kube_port() -> Option<String> {
    env::var(SERVICE_PORTENV).ok()
}

/// Returns token from specified path in cluster.
pub fn load_token() -> Result<String> {
    utils::read_file_to_string(SERVICE_TOKENFILE)
}

/// Returns certification from specified path in cluster.
pub fn load_cert() -> Result<Vec<Vec<u8>>> {
    Ok(utils::certs(&utils::read_file(SERVICE_CERTFILE)?))
}

/// Returns the default namespace from specified path in cluster.
pub fn load_default_ns() -> Result<String> {
    utils::read_file_to_string(SERVICE_DEFAULT_NS)
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
