use http::{Request, header::ACCEPT};

#[cfg(test)] mod tests;

/// Add the drop parameter while preserving the original representation as a fallback.
pub(crate) fn prepare(request: &mut Request<Vec<u8>>, drop_managed_fields: bool) {
    if !drop_managed_fields {
        return;
    }
    // Preserve metadata-only negotiation, including its fallback representation.
    let accept = request
        .headers()
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    let value = format!("{accept};drop=metadata.managedFields, {accept}");
    request
        .headers_mut()
        .insert(ACCEPT, value.parse().expect("valid generated Accept header"));
}
