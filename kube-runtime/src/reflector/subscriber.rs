//! Per-subscriber identity for the reflector's shared stream

use std::sync::atomic::{AtomicU64, Ordering};

/// Identifier for a subscriber of a reflector.
pub(crate) type SubscriberId = u64;

static NEXT_SUBSCRIBER_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a fresh subscriber id, unique for the duration of the process.
pub(crate) fn fresh_subscriber_id() -> SubscriberId {
    NEXT_SUBSCRIBER_ID.fetch_add(1, Ordering::Relaxed)
}
