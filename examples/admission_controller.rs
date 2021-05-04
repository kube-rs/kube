use std::{
    convert::{Infallible, TryInto},
    error::Error,
};

use kube::api::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject,
};
#[macro_use] extern crate log;
use warp::{reply, Filter, Reply};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();

    let routes = warp::path("mutate")
        .and(warp::body::json())
        .and_then(mutate_handler)
        .with(warp::trace::request());

    // You must generate a certificate for the service / url,
    // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
    // See admission_setup.sh + admission_controller.yaml.tpl
    // https://kubernetes.io/blog/2019/03/21/a-guide-to-kubernetes-admission-controllers/#tls-certificates
    let addr = format!("{}:8443", std::env::var("ADMISSION_PRIVATE_IP").unwrap());
    warp::serve(warp::post().and(routes))
        .tls()
        .cert_path("admission-controller-tls.crt")
        .key_path("admission-controller-tls.key")
        //.run(([0, 0, 0, 0], 8443))
        .run(addr.parse::<std::net::SocketAddr>().unwrap())
        .await;
}

async fn mutate_handler(body: AdmissionReview<DynamicObject>) -> Result<impl Reply, Infallible> {
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Ok(reply::json(
                &AdmissionResponse::invalid(err.to_string()).into_review(),
            ));
        }
    };

    let mut res = AdmissionResponse::from(&req);

    if let Some(obj) = req.object {
        info!(
            "got request id {} to {:?} resource {}",
            req.uid.clone(),
            req.operation,
            obj.metadata.name.as_ref().unwrap_or(&"unknown".to_owned())
        );

        res = match mutate(res.clone(), obj, req.dry_run) {
            Ok(res) => res,
            Err(err) => {
                error!("mutate failed: {}", err.to_string());
                res.deny(err.to_string())
            }
        };
    };
    // Need to return an AdmissionResponse wrapped in an AdmissionReview
    Ok(reply::json(&res.into_review()))
}

fn mutate(
    mut res: AdmissionResponse,
    obj: DynamicObject,
    _dry_run: bool, // no external operations performed, no need to track dry_run
) -> Result<AdmissionResponse, Box<dyn Error>> {
    let labels = obj.metadata.labels.unwrap_or_default();

    // If the resource contains an "illegal" label, we reject it
    if labels.contains_key("illegal") {
        return Err("Resource contained 'illegal' label".into());
    }

    // If the resource doesn't contain "admission", we add it to the resource.
    let patches = if !labels.contains_key("admission") {
        use json_patch::{AddOperation, PatchOperation};
        vec![
            // Ensure labels exist before adding a key to it
            PatchOperation::Add(AddOperation {
                path: "/metadata/labels".to_string(),
                value: serde_json::json!({}),
            }),
            // Add our label
            PatchOperation::Add(AddOperation {
                path: "/metadata/labels/admission".to_owned(),
                value: serde_json::Value::String("modified-by-admission-controller".to_owned()),
            }),
        ]
    } else {
        vec![]
    };

    if !patches.is_empty() {
        res = res.with_patch(json_patch::Patch(patches))?;
    }

    Ok(res)
}
