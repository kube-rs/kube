use json_patch::jsonptr::PointerBuf;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, Resource, ResourceExt,
};
use rustls::pki_types::pem::PemObject;
use std::{convert::Infallible, error::Error};
use tokio_rustls::rustls;
use tracing::*;
use warp::{reply, Filter, Reply};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let routes = warp::path("mutate")
        .and(warp::body::json())
        .and_then(mutate_handler)
        .with(warp::trace::request());

    // You must generate a certificate for the service / url,
    // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
    // See admission_setup.sh + admission_controller.yaml.tpl for how to do this.
    let addr = format!("{}:8443", std::env::var("ADMISSION_PRIVATE_IP").unwrap());
    let addr = addr.parse::<std::net::SocketAddr>().unwrap();
    let tcp = tokio::net::TcpListener::bind(addr).await.unwrap();

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            rustls::pki_types::CertificateDer::pem_file_iter("admission-controller-tls.crt")
                .unwrap()
                .map(|cert| cert.unwrap())
                .collect::<Vec<_>>(),
            rustls::pki_types::PrivateKeyDer::from_pem_file("admission-controller-tls.key").unwrap(),
        )
        .unwrap();
    let tls_acceptor = tokio_rustls::server::TlsAcceptor::from(std::sync::Arc::new(tls_config));

    let service = warp::service(warp::post().and(routes));
    let service = hyper_util::service::TowerToHyperService::new(service);

    loop {
        let (tcp_sock, remote_addr) = match tcp.accept().await {
            Ok(t) => t,
            Err(e) => {
                error!("couldn't accept connection: {}", e);
                break;
            }
        };
        let tls_acceptor = tls_acceptor.clone();
        let plain_sock = match tls_acceptor.accept(tcp_sock).await {
            Ok(sock) => sock,
            Err(e) => {
                warn!("failed to open tls connection with {}: {}", remote_addr, e);
                continue;
            }
        };
        let plain_sock = hyper_util::rt::tokio::TokioIo::new(plain_sock);
        let service = service.clone();
        tokio::spawn(async move {
            let conn_res = hyper::server::conn::http1::Builder::new()
                .serve_connection(plain_sock, service)
                .await;
            if let Err(e) = conn_res {
                warn!("error while handling connection for {}: {}", remote_addr, e);
            };
        });
    }
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
