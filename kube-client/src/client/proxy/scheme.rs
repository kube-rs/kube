use std::net::SocketAddr;

use http::{uri::Scheme, HeaderValue};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown proxy scheme")]
    UnknownScheme,
}

#[derive(Clone)]
pub enum ProxyScheme {
    Http {
        uri: http::Uri,
        auth: Option<HeaderValue>,
    },

    Https {
        uri: http::Uri,
        auth: Option<HeaderValue>,
    },

    #[cfg(feature = "socks-proxy")]
    Socks5 {
        addr: SocketAddr,
        auth: Option<(String, String)>,
        remote_dns: bool,
    },
}

impl ProxyScheme {
    pub fn http(uri: http::Uri) -> Self {
        Self::Http { uri, auth: None }
    }

    pub fn https(uri: http::Uri) -> Self {
        Self::Https { uri, auth: None }
    }

    fn from_uri(uri: http::Uri) -> Result<Self, Error> {
        match uri.scheme().map(|s| s.as_str()) {
            Some("https") => {}
            // No shema is assumed to be http like Go
            Some("http") | None => {}

            #[cfg(feature = "socks-proxy")]
            Some("socks5") => {}
            #[cfg(feature = "socks-proxy")]
            Some("socks5h") => {}

            _ => return Err(Error::UnknownScheme),
        }
        todo!();
    }
}
