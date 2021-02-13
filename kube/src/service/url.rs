use http::Request;
use hyper::Body;

/// Set cluster URL
///
/// This also propagates `Config::default_ns` if requested via `Api::default_namespaced`
/// `"DEFAULT_NS"` is the placeholder from `Api::default_namespaced` which it's kept up to date with
/// This cannot clash with a legal namespace as it contains an underscore
pub fn set_cluster_url(req: Request<Body>, url: &url::Url, default_ns: &str) -> Request<Body> {
    let (mut parts, body) = req.into_parts();
    let pandq = parts.uri.path_and_query().expect("valid path+query from kube");
    parts.uri = finalize_url(url, &pandq)
        .replace("DEFAULT_NS", default_ns)
        .parse()
        .expect("valid URL");
    Request::from_parts(parts, body)
}

/// An internal url joiner to deal with the two different interfaces
///
/// - api module produces a http::Uri which we can turn into a PathAndQuery (has a leading slash by construction)
/// - config module produces a url::Url from user input (sometimes contains path segments)
///
/// This deals with that in a pretty easy way (tested below)
fn finalize_url(cluster_url: &url::Url, request_pandq: &http::uri::PathAndQuery) -> String {
    let base = cluster_url.as_str().trim_end_matches('/'); // pandq always starts with a slash
    format!("{}{}", base, request_pandq)
}

#[cfg(test)]
mod tests {
    #[test]
    fn normal_host() {
        let minikube_host = "https://192.168.1.65:8443";
        let cluster_url = url::Url::parse(minikube_host).unwrap();
        let apipath: http::Uri = "/api/v1/nodes?hi=yes".parse().unwrap();
        let pandq = apipath.path_and_query().expect("could pandq apipath");
        let final_url = super::finalize_url(&cluster_url, &pandq);
        assert_eq!(
            final_url.as_str(),
            "https://192.168.1.65:8443/api/v1/nodes?hi=yes"
        );
    }

    #[test]
    fn rancher_host() {
        // in rancher, kubernetes server names are not hostnames, but a host with a path:
        let rancher_host = "https://hostname/foo/bar";
        let cluster_url = url::Url::parse(rancher_host).unwrap();
        assert_eq!(cluster_url.host_str().unwrap(), "hostname");
        assert_eq!(cluster_url.path(), "/foo/bar");
        // we must be careful when using Url::join on our http::Uri result
        // as a straight two Uri::join would trim away rancher's initial path
        // case in point (discards original path):
        assert_eq!(cluster_url.join("/api/v1/nodes").unwrap().path(), "/api/v1/nodes");

        let apipath: http::Uri = "/api/v1/nodes?hi=yes".parse().unwrap();
        let pandq = apipath.path_and_query().expect("could pandq apipath");

        let final_url = super::finalize_url(&cluster_url, &pandq);
        assert_eq!(final_url.as_str(), "https://hostname/foo/bar/api/v1/nodes?hi=yes");
    }
}
