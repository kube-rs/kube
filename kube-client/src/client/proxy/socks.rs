use std::{
    io,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
};

use http::uri::Scheme;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use super::BoxError;


pub(super) enum DnsResolve {
    Local,
    // `socks5h://` Note that Go http proxy only supports `socks5://`
    Proxy,
}

pub(super) async fn connect(
    socket_addr: SocketAddr,
    dst: http::Uri,
    dns: DnsResolve,
) -> Result<TcpStream, BoxError> {
    let https = dst.scheme() == Some(&Scheme::HTTPS);
    let original_host = dst
        .host()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no host in url"))?;
    let mut host = original_host.to_owned();
    let port = match dst.port_u16() {
        Some(p) => p,
        None if https => 443u16,
        _ => 80u16,
    };

    if let DnsResolve::Local = dns {
        let maybe_new_target = (host.as_str(), port).to_socket_addrs()?.next();
        if let Some(new_target) = maybe_new_target {
            host = new_target.ip().to_string();
        }
    }

    // Get a Tokio TcpStream
    let stream = Socks5Stream::connect(socket_addr, (host.as_str(), port))
        .await
        .map_err(|e| format!("socks connect error: {}", e))?;
    Ok(stream.into_inner())
}

pub fn socket_addrs(
    uri: http::Uri,
    default_port_number: impl Fn() -> Option<u16>,
) -> std::io::Result<Vec<SocketAddr>> {
    // if host is ipv6 (within `[` and `]`), vec![(ip, port).into()]
    // if host is ipv4, vec![(ip, port).into()],
    // if host is domain, (domain, port).to_socket_addrs()?.collect(),
    let host = uri
        .host()
        .ok_or_else(|| std::io::Error::new(io::ErrorKind::InvalidData, "No host name"))?;
    let port = uri
        .port_u16()
        .or_else(default_port_number)
        .ok_or_else(|| std::io::Error::new(io::ErrorKind::InvalidData, "No port number"))?;

    if host.as_bytes()[0] == b'[' {
        // IPv6 in brackets
        match (&host[1..host.len() - 1]).parse::<IpAddr>() {
            Ok(IpAddr::V4(_)) => unreachable!("Valid uri cannot have ipv4 within brackets"),
            Ok(IpAddr::V6(addr)) => Ok(vec![(addr, port).into()]),
            Err(err) => Err(std::io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid ipv6: {}", err),
            )),
        }
    } else {
        match host.parse::<IpAddr>() {
            Ok(IpAddr::V4(addr)) => Ok(vec![(addr, port).into()]),
            Ok(IpAddr::V6(_)) => unreachable!("Valid uri cannot have ipv6 without brackets"),
            // If it's not ipv4, then it's a domain. Valid because it's from a avalid Uri.
            Err(_) => Ok((host, port).to_socket_addrs()?.collect()),
        }
    }
}
