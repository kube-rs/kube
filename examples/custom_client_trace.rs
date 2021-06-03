// Custom client example with TraceLayer.
use std::time::Duration;

use http::{Request, Response};
use hyper::{client::HttpConnector, Body};
use hyper_tls::HttpsConnector;
use k8s_openapi::api::core::v1::ConfigMap;
use tower::ServiceBuilder;
use tower_http::{decompression::DecompressionLayer, trace::TraceLayer};
use tracing::Span;

use kube::{
    api::{Api, ListParams},
    service::SetBaseUriLayer,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug,custom_client_trace=debug");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;
    // Create HttpsConnector using `native_tls::TlsConnector` based on `Config`.
    let https = {
        let tls = tokio_native_tls::TlsConnector::from(config.native_tls_connector()?);
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        HttpsConnector::from((http, tls))
    };
    let client = Client::new(
        ServiceBuilder::new()
            .layer(SetBaseUriLayer::new(config.cluster_url))
            // Add `DecompressionLayer` to make request headers interesting.
            .layer(DecompressionLayer::new())
            .layer(
                // Attribute names follow [Semantic Conventions].
                // [Semantic Conventions]: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/semantic_conventions/http.md#http-client
                TraceLayer::new_for_http()
                    .make_span_with(|request: &Request<Body>| {
                        tracing::debug_span!(
                            "HTTP",
                            http.method = %request.method(),
                            http.url = %request.uri(),
                            http.status_code = tracing::field::Empty,
                            otel.name = %format!("HTTP {}", request.method()),
                            otel.kind = "client",
                            otel.status_code = tracing::field::Empty,
                        )
                    })
                    .on_request(|request: &Request<Body>, _span: &Span| {
                        tracing::debug!("payload: {:?} headers: {:?}", request.body(), request.headers())
                    })
                    .on_response(|response: &Response<Body>, latency: Duration, span: &Span| {
                        let status = response.status();
                        span.record("http.status_code", &status.as_u16());
                        if status.is_client_error() || status.is_server_error() {
                            span.record("otel.status_code", &"ERROR");
                        }
                        tracing::debug!("finished in {}ms", latency.as_millis())
                    }),
            )
            .service(hyper::Client::builder().build(https)),
    );

    let cms: Api<ConfigMap> = Api::namespaced(client, "default");
    for cm in cms.list(&ListParams::default()).await? {
        println!("{:?}", cm);
    }

    Ok(())
}
