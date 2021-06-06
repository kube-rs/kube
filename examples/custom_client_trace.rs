// Custom client example with TraceLayer.
use std::time::Duration;

use http::{Request, Response};
use hyper::Body;
use k8s_openapi::api::core::v1::Pod;
use tower::ServiceBuilder;
use tower_http::{decompression::DecompressionLayer, trace::TraceLayer};
use tracing::Span;

use kube::{
    Api, ResourceExt,
    client::ConfigExt,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug,custom_client_trace=debug");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;
    let https = config.native_tls_https_connector()?;
    let client = Client::new(
        ServiceBuilder::new()
            .layer(config.base_uri_layer())
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
    ).with_default_namespace(config.default_ns);

    let pods: Api<Pod> = Api::default_namespaced(client);
    for p in pods.list(&Default::default()).await? {
        println!("{}", p.name());
    }

    Ok(())
}
