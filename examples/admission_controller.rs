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
    std::env::set_var("RUST_LOG", "debug,kube=debug");
    tracing_subscriber::fmt::init();

    let routes = warp::path("mutate")
        .and(warp::body::json())
        .and_then(mutate_handler)
        .with(warp::trace::request());


    // For admission controllers running behind a service, you must generate a
    // certificate for the service and encode the CA in the
    // MutatingWebhookConfiguration, then terminate TLS here. For controllers
    // running outside of the cluster, while still necessary, it's fine to
    // handle TLS upstream. See:
    // https://kubernetes.io/blog/2019/03/21/a-guide-to-kubernetes-admission-controllers/#tls-certificates

    // warp::serve(warp::post().and(routes))
    //     .tls()
    //     .cert_path("admission-controller-tls.crt")
    //     .key_path("admission-controller-tls.key")
    //     .run(([0, 0, 0, 0], 8443))
    //     .await;

    warp::serve(warp::post().and(routes))
        .run(([0, 0, 0, 0], 8080))
        .await;
}

async fn mutate_handler(body: AdmissionReview<DynamicObject>) -> Result<impl Reply, Infallible> {
    let req: AdmissionRequest<DynamicObject> = match body.try_into() {
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

    Ok(reply::json(&res.into_review()))
}

fn mutate(
    mut res: AdmissionResponse,
    obj: DynamicObject,
    dry_run: bool,
) -> Result<AdmissionResponse, Box<dyn Error>> {
    let mut patches: Vec<json_patch::PatchOperation> = Vec::new();
    let labels = obj.metadata.labels.unwrap_or_default();

    // If the resource contains a label named "test_reject", it will be
    // forbidden from creating.
    if labels.contains_key("test_reject") {
        return Err("Resource contained 'test_reject' label".into());
    }

    // If the resource doesn't contain "test_modify", we add it to the
    // resource.
    if !labels.contains_key("test_modify") {
        patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
            path: "/metadata/labels/test_modify".to_owned(),
            value: serde_json::Value::String("modified-by-admission-controller".to_owned()),
        }))
    }

    if !dry_run && !patches.is_empty() {
        res = res.with_patch(json_patch::Patch(patches))?;
    }

    Ok(res)
}
