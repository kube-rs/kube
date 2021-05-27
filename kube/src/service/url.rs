use http::{uri, Request};
use hyper::Body;

/// Set cluster URL.
pub fn set_cluster_url(req: Request<Body>, base_uri: &http::Uri) -> Request<Body> {
    let (mut parts, body) = req.into_parts();
    let request_pandq = parts.uri.path_and_query().expect("nonempty path+query");
    parts.uri = finalize_url(base_uri, request_pandq);
    Request::from_parts(parts, body)
}

// Join base URI and Path+Query, preserving any path in the base.
fn finalize_url(base_uri: &http::Uri, request_pandq: &uri::PathAndQuery) -> http::Uri {
    let mut builder = uri::Builder::new();
    if let Some(scheme) = base_uri.scheme() {
        builder = builder.scheme(scheme.as_str());
    }
    if let Some(authority) = base_uri.authority() {
        builder = builder.authority(authority.as_str());
    }
    if let Some(pandq) = base_uri.path_and_query() {
        // If `base_uri` has path, remove any trailing space and join.
        // `PathAndQuery` always starts with a slash.
        let base_path = pandq.path().trim_end_matches('/');
        builder = builder.path_and_query(format!("{}{}", base_path, request_pandq));
    } else {
        builder = builder.path_and_query(request_pandq.as_str());
    }
    builder.build().expect("valid URI")
}

#[cfg(test)]
mod tests {
    #[test]
    fn normal_host() {
        let base_uri = http::Uri::from_static("https://192.168.1.65:8443");
        let apipath = http::Uri::from_static("/api/v1/nodes?hi=yes");
        let pandq = apipath.path_and_query().expect("could pandq apipath");
        assert_eq!(
            super::finalize_url(&base_uri, &pandq),
            "https://192.168.1.65:8443/api/v1/nodes?hi=yes"
        );
    }

    #[test]
    fn rancher_host() {
        // in rancher, kubernetes server names are not hostnames, but a host with a path:
        let base_uri = http::Uri::from_static("https://example.com/foo/bar");
        let api_path = http::Uri::from_static("/api/v1/nodes?hi=yes");
        let pandq = api_path.path_and_query().unwrap();
        assert_eq!(
            super::finalize_url(&base_uri, &pandq),
            "https://example.com/foo/bar/api/v1/nodes?hi=yes"
        );
    }
}
