use http::{self, Response, StatusCode};
use hyper::Body;
use thiserror::Error;
use tokio_tungstenite::tungstenite as ws;

// Binary subprotocol v4. See `Client::connect`.
pub const WS_PROTOCOL: &str = "v4.channel.k8s.io";

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
pub fn verify_response(res: &Response<Body>, key: &str) -> Result<(), UpgradeConnectionError> {
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

    // Make sure that the server returned the correct subprotocol.
    if !headers
        .get(http::header::SEC_WEBSOCKET_PROTOCOL)
        .map(|h| h == WS_PROTOCOL)
        .unwrap_or(false)
    {
        return Err(UpgradeConnectionError::SecWebSocketProtocolMismatch);
    }

    Ok(())
}

/// Generate a random key for the `Sec-WebSocket-Key` header.
/// This must be nonce consisting of a randomly selected 16-byte value in base64.
pub fn sec_websocket_key() -> String {
    let r: [u8; 16] = rand::random();
    base64::encode(r)
}
