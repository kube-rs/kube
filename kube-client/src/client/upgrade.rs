use http::{self, HeaderValue, Response, StatusCode};
use thiserror::Error;
use tokio_tungstenite::tungstenite as ws;

use crate::{client::Body, Error, Result};

#[derive(Debug)]
pub enum StreamProtocol {
    /// Binary subprotocol v4. See `Client::connect`.
    V4,

    /// Binary subprotocol v5. See `Client::connect`.
    /// v5 supports CLOSE signals.
    /// https://github.com/kubernetes/kubernetes/blob/master/staging/src/k8s.io/apimachinery/pkg/util/remotecommand/constants.go#L52C26-L52C43
    V5,
}

impl StreamProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V4 => "v4.channel.k8s.io",
            Self::V5 => "v5.channel.k8s.io",
        }
    }

    fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    pub fn supports_stream_close(&self) -> bool {
        match self {
            Self::V4 => false,
            Self::V5 => true,
        }
    }

    /// Add HTTP header SEC_WEBSOCKET_PROTOCOL with a list of supported protocol.
    pub fn add_to_headers(headers: &mut http::HeaderMap) -> Result<()> {
        // Protocols we support in our preferred order.
        let supported_protocols = [
            // v5 supports CLOSE signals.
            Self::V5.as_str(),
            // Use the binary subprotocol v4, to get JSON `Status` object in `error` channel (3).
            // There's no official documentation about this protocol, but it's described in
            // [`k8s.io/apiserver/pkg/util/wsstream/conn.go`](https://git.io/JLQED).
            // There's a comment about v4 and `Status` object in
            // [`kublet/cri/streaming/remotecommand/httpstream.go`](https://git.io/JLQEh).
            Self::V4.as_str(),
        ];

        let header_value_string = supported_protocols.join(", ");

        // Note: Multiple headers does not work. Only a single CSV works.
        headers.insert(
            http::header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_str(&header_value_string).map_err(|e| Error::HttpError(e.into()))?,
        );

        Ok(())
    }

    /// Return the subprotocol of an HTTP response.
    fn get_from_response<B>(res: &Response<B>) -> Option<Self> {
        let headers = res.headers();

        match headers
            .get(http::header::SEC_WEBSOCKET_PROTOCOL)
            .map(|h| h.as_bytes())
        {
            Some(protocol) => {
                if protocol == Self::V4.as_bytes() {
                    Some(Self::V4)
                } else if protocol == Self::V5.as_bytes() {
                    Some(Self::V5)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Possible errors from upgrading to a WebSocket connection
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
#[derive(Debug, Error)]
pub enum UpgradeConnectionError {
    /// The server did not respond with [`SWITCHING_PROTOCOLS`] status when upgrading the
    /// connection.
    ///
    /// [`SWITCHING_PROTOCOLS`]: http::status::StatusCode::SWITCHING_PROTOCOLS
    #[error("failed to switch protocol: {0}")]
    ProtocolSwitch(http::status::StatusCode),

    /// `Upgrade` header was not set to `websocket` (case insensitive)
    #[error("upgrade header was not set to websocket")]
    MissingUpgradeWebSocketHeader,

    /// `Connection` header was not set to `Upgrade` (case insensitive)
    #[error("connection header was not set to Upgrade")]
    MissingConnectionUpgradeHeader,

    /// `Sec-WebSocket-Accept` key mismatched.
    #[error("Sec-WebSocket-Accept key mismatched")]
    SecWebSocketAcceptKeyMismatch,

    /// `Sec-WebSocket-Protocol` mismatched.
    #[error("Sec-WebSocket-Protocol mismatched")]
    SecWebSocketProtocolMismatch,

    /// Failed to get pending HTTP upgrade.
    #[error("failed to get pending HTTP upgrade: {0}")]
    GetPendingUpgrade(#[source] hyper::Error),
}

// Verify upgrade response according to RFC6455.
// Based on `tungstenite` and added subprotocol verification.
pub fn verify_response(res: &Response<Body>, key: &str) -> Result<StreamProtocol, UpgradeConnectionError> {
    if res.status() != StatusCode::SWITCHING_PROTOCOLS {
        return Err(UpgradeConnectionError::ProtocolSwitch(res.status()));
    }

    let headers = res.headers();
    if !headers
        .get(http::header::UPGRADE)
        .and_then(|h| h.to_str().ok())
        .map(|h| h.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
    {
        return Err(UpgradeConnectionError::MissingUpgradeWebSocketHeader);
    }

    if !headers
        .get(http::header::CONNECTION)
        .and_then(|h| h.to_str().ok())
        .map(|h| h.eq_ignore_ascii_case("Upgrade"))
        .unwrap_or(false)
    {
        return Err(UpgradeConnectionError::MissingConnectionUpgradeHeader);
    }

    let accept_key = ws::handshake::derive_accept_key(key.as_ref());
    if !headers
        .get(http::header::SEC_WEBSOCKET_ACCEPT)
        .map(|h| h == &accept_key)
        .unwrap_or(false)
    {
        return Err(UpgradeConnectionError::SecWebSocketAcceptKeyMismatch);
    }

    // Make sure that the server returned an expected subprotocol.
    let protocol = match StreamProtocol::get_from_response(res) {
        Some(p) => p,
        None => return Err(UpgradeConnectionError::SecWebSocketProtocolMismatch),
    };

    Ok(protocol)
}
