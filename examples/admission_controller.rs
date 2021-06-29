use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, ResourceExt,
};
use std::{
    convert::{Infallible, TryInto},
    error::Error,
};
#[macro_use] extern crate log;
use warp::{reply, Filter, Reply};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "info,warp=warn,kube=debug");
    env_logger::init();

    let routes = warp::path("mutate")
        .and(warp::body::json())
        .and_then(mutate_handler)
        .with(warp::trace::request());

    // You must generate a certificate for the service / url,
    // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
    // See admission_setup.sh + admission_controller.yaml.tpl for how to do this.
    let addr = format!("{}:8443", std::env::var("ADMISSION_PRIVATE_IP").unwrap());
    warp::serve(warp::post().and(routes))
        .tls()
        .cert_path("admission-controller-tls.crt")
        .key_path("admission-controller-tls.key")
        //.run(([0, 0, 0, 0], 8443)) // in-cluster
        .run(addr.parse::<std::net::SocketAddr>().unwrap()) // local-dev
        .await;
}

// A general /mutate handler, handling errors from the underlying business logic
async fn mutate_handler(body: AdmissionReview<DynamicObject>) -> Result<impl Reply, Infallible> {
    // Parse incoming webhook AdmissionRequest first
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Ok(reply::json(
                &AdmissionResponse::invalid(err.to_string()).into_review(),
            ));
        }
    };

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        res = match mutate(res.clone(), &obj) {
            Ok(res) => {
                info!("accepted: {:?} on Foo {}", req.operation, obj.name());
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {} ({})", req.operation, obj.name(), err);
                res.deny(err.to_string())
            }
        };
    };
    // Wrap the AdmissionResponse wrapped in an AdmissionReview
    Ok(reply::json(&res.into_review()))
}

// The main handler and core business logic, failures here implies rejected applies
fn mutate(res: AdmissionResponse, obj: &DynamicObject) -> Result<AdmissionResponse, Box<dyn Error>> {
    // If the resource contains an "illegal" label, we reject it
    if obj.labels().contains_key("illegal") {
        return Err("Resource contained 'illegal' label".into());
    }

    // If the resource doesn't contain "admission", we add it to the resource.
    if !obj.labels().contains_key("admission") {
        let patches = vec![
            // Ensure labels exist before adding a key to it
            json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: "/metadata/labels".into(),
                value: serde_json::json!({}),
            }),
            // Add our label
            json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: "/metadata/labels/admission".into(),
                value: serde_json::Value::String("modified-by-admission-controller".into()),
            }),
        ];
        Ok(res.with_patch(json_patch::Patch(patches))?)
    } else {
        Ok(res)
    }
}
