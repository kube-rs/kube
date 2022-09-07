use thiserror::Error;

// Mounted credential files
const SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
const SERVICE_CERTFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";
const SERVICE_DEFAULT_NS: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

/// Errors from loading in-cluster config
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to read the default namespace for the service account
    #[error("failed to read the default namespace: {0}")]
    ReadDefaultNamespace(#[source] std::io::Error),

    /// Failed to read a certificate bundle
    #[error("failed to read a certificate bundle: {0}")]
    ReadCertificateBundle(#[source] std::io::Error),

    /// Failed to parse cluster port value
    #[error("failed to parse cluster port: {0}")]
    ParseClusterPort(#[source] std::num::ParseIntError),

    /// Failed to parse cluster url
    #[error("failed to parse cluster url: {0}")]
    ParseClusterUrl(#[source] http::uri::InvalidUri),

    /// Failed to parse PEM-encoded certificates
    #[error("failed to parse PEM-encoded certificates: {0}")]
    ParseCertificates(#[source] pem::PemError),
}

pub fn kube_dns() -> http::Uri {
    http::Uri::from_static("https://kubernetes.default.svc/")
}

pub fn try_kube_from_legacy_env_or_dns() -> Result<http::Uri, Error> {
    // client-go requires that both environment variables are set, so we do too.
    let host = match std::env::var("KUBERNETES_SERVICE_HOST") {
        Ok(h) => h,
        Err(_) => return Ok(kube_dns()),
    };
    let port = match std::env::var("KUBERNETES_SERVICE_PORT") {
        Ok(p) => p.parse::<u16>().map_err(Error::ParseClusterPort)?,
        Err(_) => return Ok(kube_dns()),
    };

    // Format a host and, if not using 443, a port.
    //
    // Ensure that IPv6 addresses are properly bracketed.
    const HTTPS: &str = "https";
    let uri = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => {
            if port == 443 {
                if ip.is_ipv6() {
                    format!("{HTTPS}://[{ip}]")
                } else {
                    format!("{HTTPS}://{ip}")
                }
            } else {
                let addr = std::net::SocketAddr::new(ip, port);
                format!("{HTTPS}://{addr}")
            }
        }
        Err(_) => {
            if port == 443 {
                format!("{HTTPS}://{host}")
            } else {
                format!("{HTTPS}://{host}:{port}")
            }
        }
    };

    uri.parse().map_err(Error::ParseClusterUrl)
}

pub fn token_file() -> String {
    SERVICE_TOKENFILE.to_owned()
}

/// Returns certification from specified path in cluster.
pub fn load_cert() -> Result<Vec<Vec<u8>>, Error> {
    let certs = std::fs::read(SERVICE_CERTFILE).map_err(Error::ReadCertificateBundle)?;
    super::certs(&certs).map_err(Error::ParseCertificates)
}

/// Returns the default namespace from specified path in cluster.
pub fn load_default_ns() -> Result<String, Error> {
    std::fs::read_to_string(SERVICE_DEFAULT_NS).map_err(Error::ReadDefaultNamespace)
}
