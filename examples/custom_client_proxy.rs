use http::Uri;
use hyper::client::HttpConnector;
use k8s_openapi::api::core::v1::ConfigMap;
use tower::ServiceBuilder;

use kube::{
    api::{Api, ListParams},
    client::{ConfigExt, ProxyConnector},
    Client, Config,
};

/*
// Need to set `client_certs` so that `mitmproxy` can make requests as an authorized user.
//
// Store client certs and key as PEM (adjust the query for your user):
```bash
kubectl config view \
    --raw \
    -o jsonpath='{.users[?(@.name == "admin@k3d-dev")].user.client-certificate-data}' \
| base64 -d \
> client-certs.pem

kubectl config view \
    --raw \
    -o jsonpath='{.users[?(@.name == "admin@k3d-dev")].user.client-key-data}' \
| base64 -d \
>> client-certs.pem
```

// `--ssl-insecure` is necessary because the API server uses self signed certificates:
```bash
mitmproxy -p 5000 --ssl-insecure --set client_certs=$(pwd)/client-certs.pem
# or
mitmweb -p 5000 --ssl-insecure --set client_certs=$(pwd)/client-certs.pem
```
// After running this example, you should be able to inspect
// `GET /api/v1/namespaces/default/configmaps`
 */

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "trace");
    // TODO Client should use ProxyConnector based on environment variables or proxy_url
    // std::env::set_var("HTTPS_PROXY", "http://localhost:5000");
    tracing_subscriber::fmt::init();

    let mut config = Config::infer().await?;
    config.accept_invalid_certs = true;
    let connector = {
        let tls = config.native_tls_connector()?;
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        let proxy_url = "http://localhost:5000".parse::<Uri>().unwrap();
        ProxyConnector::native_tls(proxy_url, http, tls)
    };

    let service = ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .service(hyper::Client::builder().build(connector));
    let client = Client::new(service, config.default_namespace);

    let cms: Api<ConfigMap> = Api::namespaced(client, "default");
    for cm in cms.list(&ListParams::default()).await? {
        println!("{:?}", cm);
    }

    Ok(())
}
