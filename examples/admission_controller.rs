use axum::{response::IntoResponse, routing::post, Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use json_patch::jsonptr::PointerBuf;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, Resource, ResourceExt,
};
use std::{error::Error, net::SocketAddr};
use tower_http::trace::TraceLayer;
use tracing::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/mutate", post(mutate_handler)).layer(
        TraceLayer::new_for_http()
            .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(Level::INFO)),
    );

    // You must generate a certificate for the service / url,
    // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
    // See admission_setup.sh + admission_controller.yaml.tpl for how to do this.
    let addr = format!("{}:8443", std::env::var("ADMISSION_PRIVATE_IP").unwrap());
    axum_server::bind_rustls(
        // SocketAddr::from(([0, 0, 0, 0], 8443)), // in-cluster
        addr.parse::<SocketAddr>().unwrap(), // local-dev
        RustlsConfig::from_pem_file("admission-controller-tls.crt", "admission-controller-tls.key")
            .await
            .unwrap(),
    )
    .serve(app.into_make_service())
    .await
    .unwrap();
}

// A general /mutate handler, handling errors from the underlying business logic
async fn mutate_handler(
    Json(body): Json<AdmissionReview<DynamicObject>>,
) -> Json<AdmissionReview<DynamicObject>> {
    // Parse incoming webhook AdmissionRequest first
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Json(AdmissionResponse::invalid(err.to_string()).into_review());
        }
    };

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        let name = obj.name_any(); // apiserver may not have generated a name yet
        let kind = obj.types.clone().unwrap_or_default().kind;
        res = match mutate(res.clone(), &obj) {
            Ok(res) => {
                info!("accepted: {:?} on {kind}/{name}", req.operation);
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {kind}/{name} ({})", req.operation, err);
                res.deny(err.to_string())
            }
        };
    };
    // Wrap the AdmissionResponse wrapped in an AdmissionReview
    Json(res.into_review())
}

// The main handler and core business logic, failures here implies rejected applies
fn mutate(res: AdmissionResponse, obj: &DynamicObject) -> Result<AdmissionResponse, Box<dyn Error>> {
    // If the resource contains an "illegal" label, we reject it
    if obj.labels().contains_key("illegal") {
        return Err("Resource contained 'illegal' label".into());
    }

    // If the resource doesn't contain "admission", we add it to the resource.
    if !obj.labels().contains_key("admission") {
        let mut patches = Vec::new();

        // Ensure labels exist before adding a key to it
        if obj.meta().labels.is_none() {
            patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: PointerBuf::from_tokens(["metadata", "labels"]),
                value: serde_json::json!({}),
            }));
        }
        // Add our label
        patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
            path: PointerBuf::from_tokens(["metadata", "labels", "admission"]),
            value: serde_json::Value::String("modified-by-admission-controller".into()),
        }));
        Ok(res.with_patch(json_patch::Patch(patches))?)
    } else {
        Ok(res)
    }
}
