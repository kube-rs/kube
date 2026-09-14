use std::{net::IpAddr, str::FromStr};

use ipnet::IpNet;
use thiserror::Error;

use crate::util::nonempty;

/// This error enum contains all variants we can hit during `NO_PROXY`/`no_proxy` parsing and
/// validation.
#[derive(Debug, Error)]
pub enum Error {
    /// Comma-separated value is empty
    #[error("comma-separated value is empty")]
    EmptyValue,

    /// Failed to parse the input as any of the valid/supported matchers
    #[error("failed to parse {input:?} as matcher. supported matchers are: cidr, ip address, and host")]
    ParseMatcher {
        /// The input which failed to parse
        input: String,
    },

    /// The parsed URI doesn't contain a host (it is a relative URI)
    #[error("failed to find valid host in parsed uri \"{uri}\"")]
    NoHost {
        /// The URI not containing a host
        uri: http::Uri,
    },
}

/// A struct containing a NO_PROXY/no_proxy configuration.
///
/// [`NoProxy::matches`] checks if a target URI matches its configuration.
#[derive(Debug)]
pub enum NoProxy {
    /// Every proxy configured via env var of kube config will be ignored.
    Wildcard,

    /// Only a selected list of targets is ignored.
    Matchers(Vec<NoProxyMatcher>),
}

impl FromStr for NoProxy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            return Ok(Self::Wildcard);
        }

        // Individual targets are separated by a comma. Whitespace-separated strings
        // are not supported.
        let parts: Vec<&str> = s.split(',').collect();
        let mut matchers = Vec::new();

        for part in parts {
            // Trim both the start and the end to get rid of any leading or trailing whitespace.
            let matcher = NoProxyMatcher::from_str(part.trim())?;
            matchers.push(matcher);
        }

        Ok(Self::Matchers(matchers))
    }
}

impl NoProxy {
    /// Parse the configuration from the `NO_PROXY` and `no_proxy` environment variables.
    /// `no_proxy` takes precedence.
    ///
    /// If both environment variables are set but are empty, this function returns `Ok(None)`.
    pub fn from_env() -> Result<Option<Self>, Error> {
        if let Some(raw) =
            nonempty(std::env::var("NO_PROXY").ok()).or_else(|| nonempty(std::env::var("no_proxy").ok()))
        {
            Ok(Some(Self::from_str(&raw)?))
        } else {
            Ok(None)
        }
    }

    /// Checks if the [`target`] is matched by this configuration.
    pub fn matches(&self, target: &http::Uri) -> Result<bool, Error> {
        match self {
            NoProxy::Wildcard => Ok(true),
            NoProxy::Matchers(matchers) => {
                let host = Host::try_from(target)?;
                let target_port = port_via_scheme(target);

                for matcher in matchers {
                    if host.matches(target_port, matcher) {
                        return Ok(true);
                    } else {
                        continue;
                    }
                }

                Ok(false)
            }
        }
    }
}

/// Supported matchers.
///
/// Currently supported are:
///
/// - single IP addresses, eg. `10.0.0.1`
/// - IP address ranges in CIDR notation, eg. `10.0.0.0/8`
/// - Hosts, eg. `*.example.org:8443`, `.example.org:443`, `example.org`
///
/// The wildcard matcher (`*`) is handled by [`NoProxy`] directly.
#[derive(Debug)]
pub enum NoProxyMatcher {
    IpAddr(IpAddr),
    IpNet(IpNet),
    Host {
        name: String,
        port: Option<u16>,
        exact: bool,
    },
}

impl FromStr for NoProxyMatcher {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::EmptyValue);
        }

        if let Ok(ip_net) = IpNet::from_str(s) {
            return Ok(NoProxyMatcher::IpNet(ip_net));
        }

        if let Ok(ip_addr) = IpAddr::from_str(s) {
            return Ok(NoProxyMatcher::IpAddr(ip_addr));
        }

        if let Ok(uri) = http::Uri::from_str(s) {
            let Some(mut host) = uri.host() else {
                return Err(Error::NoHost { uri });
            };

            let port = port_via_scheme(&uri);

            // *. and . behave the same. Remove the leading * if it is there
            if host.starts_with("*.") {
                host = &host[1..];
            }

            // Don't use exact matching. This will only match sub-domains of the host.
            if host.starts_with('.') {
                return Ok(NoProxyMatcher::Host {
                    name: host.to_owned(),
                    port,
                    exact: false,
                });
            }

            // Use exact matching. This will only match the full domain.
            return Ok(NoProxyMatcher::Host {
                name: host.to_owned(),
                port,
                exact: true,
            });
        }

        // NOTE (@Techassi): We currently hard-error if we can't parse the input as any of our
        // supported matchers. We could instead return a noop and print out a warning that the
        // input was ignored.
        Err(Error::ParseMatcher { input: s.to_owned() })
    }
}

/// Internal helper enum to turn a [`http::Uri`]'s host into an IP address or a hostname. This helps
/// us during match evaluation.
enum Host {
    IpAddr(IpAddr),
    Name(String),
}

impl TryFrom<&http::Uri> for Host {
    type Error = Error;

    fn try_from(uri: &http::Uri) -> Result<Self, Self::Error> {
        match uri.host() {
            Some(value) => {
                if let Ok(ip_addr) = IpAddr::from_str(value) {
                    return Ok(Self::IpAddr(ip_addr));
                }

                Ok(Self::Name(value.to_owned()))
            }
            // NOTE (@Techassi): We could instead use unwrap_or_default() here or ignore the error
            // above if we want to be less strict.
            None => Err(Error::NoHost { uri: uri.clone() }),
        }
    }
}

impl Host {
    /// Checks if the [`Host`] matches a target port and matcher combination.
    ///
    /// This is the internal logic for the [`NoProxy::matches`] function.
    fn matches(&self, target_port: Option<u16>, matcher: &NoProxyMatcher) -> bool {
        match &self {
            Host::IpAddr(target_ip_addr) => match matcher {
                NoProxyMatcher::IpAddr(ip_addr) => ip_addr == target_ip_addr,
                NoProxyMatcher::IpNet(ip_net) => ip_net.contains(target_ip_addr),
                _ => false,
            },
            Host::Name(target_name) => match matcher {
                NoProxyMatcher::Host { name, port, exact } => {
                    if *exact {
                        target_name == name && target_port == *port
                    } else {
                        target_name.ends_with(name) && target_port == *port
                    }
                }
                _ => false,
            },
        }
    }
}

/// Extracts the port as is if it is there, otherwise fall back to default port based on the scheme.
///
/// Returns [`None`] if there is no port set or if we failed to default based on the scheme.
fn port_via_scheme(uri: &http::Uri) -> Option<u16> {
    uri.port_u16().or_else(|| match uri.scheme_str() {
        Some("https") => Some(443),
        Some("http") => Some(80),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let no_proxy = NoProxy::from_str("");
        assert!(no_proxy.is_err());
    }

    #[test]
    fn wildcard() {
        let no_proxy = NoProxy::from_str("*").expect("static input must parse");
        assert!(matches!(no_proxy, NoProxy::Wildcard));
    }

    #[test]
    fn cidr() {
        let cidr = "10.0.0.0/8";
        let no_proxy = NoProxy::from_str(cidr).expect("static input must parse");
        let target_uri = http::Uri::from_str("10.0.0.1").expect("static input must parse");
        assert!(no_proxy.matches(&target_uri).expect("target uri is valid"));
    }

    #[test]
    fn ip_addr() {
        let ip_addr = "10.0.0.1";
        let no_proxy = NoProxy::from_str(ip_addr).expect("static input must parse");
        let target_uri = http::Uri::from_str(ip_addr).expect("static input must parse");
        assert!(no_proxy.matches(&target_uri).expect("target uri is valid"));
    }

    #[test]
    fn host() {
        let host = "example.org";
        let no_proxy = NoProxy::from_str(host).expect("static input must parse");
        let target_uri = http::Uri::from_str(host).expect("static input must parse");
        assert!(no_proxy.matches(&target_uri).expect("target uri is valid"));
    }
}
