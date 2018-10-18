use std::env;

use failure::Error;
use openssl::x509::X509;

use config::utils;

pub const SERVICE_HOSTENV: &str = "KUBERNETES_SERVICE_HOST";
pub const SERVICE_PORTENV: &str = "KUBERNETES_SERVICE_PORT";
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";

pub fn kube_server() -> Option<String> {
    let f = |(h, p)| format!("https://{}{}", h, p);
    kube_host().and_then(|h| kube_port().map(|p| f((h, p))))
}

fn kube_host() -> Option<String> {
    env::var(SERVICE_HOSTENV).ok()
}

fn kube_port() -> Option<String> {
    env::var(SERVICE_PORTENV).ok()
}

pub fn load_token() -> Result<String, Error> {
    utils::load_token_data_or_file(&None, &Some(SERVICE_TOKENFILE.to_string()))?.ok_or(format_err!(
        "Unable to load token from {}",
        SERVICE_TOKENFILE
    ))
}

pub fn load_cert() -> Result<X509, Error> {
    let ca = utils::load_data_or_file(&None, &Some(SERVICE_CERTFILE.to_string()))?
        .ok_or(format_err!("Unable to load certificate"))?;
    X509::from_pem(&ca).map_err(Error::from)
}
