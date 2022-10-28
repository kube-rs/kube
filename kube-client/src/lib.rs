//! Crate for interacting with the Kubernetes API
//!
//! This crate includes the tools for manipulating Kubernetes resources as
//! well as keeping track of those resources as they change over time
//!
//! # Example
//!
//! The following example will create a [`Pod`](k8s_openapi::api::core::v1::Pod)
//! and then watch for it to become available using a manual [`Api::watch`] call.
//!
//! ```rust,no_run
//! use futures::{StreamExt, TryStreamExt};
//! use kube_client::api::{Api, ResourceExt, ListParams, PatchParams, Patch};
//! use kube_client::Client;
//! use k8s_openapi::api::core::v1::Pod;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Read the environment to find config for kube client.
//!     // Note that this tries an in-cluster configuration first,
//!     // then falls back on a kubeconfig file.
//!     let client = Client::try_default().await?;
//!
//!     // Interact with pods in the configured namespace with the typed interface from k8s-openapi
//!     let pods: Api<Pod> = Api::default_namespaced(client);
//!
//!     // Create a Pod (cheating here with json, but it has to validate against the type):
//!     let patch: Pod = serde_json::from_value(serde_json::json!({
//!         "apiVersion": "v1",
//!         "kind": "Pod",
//!         "metadata": {
//!             "name": "my-pod"
//!         },
//!         "spec": {
//!             "containers": [
//!                 {
//!                     "name": "my-container",
//!                     "image": "myregistry.azurecr.io/hello-world:v1",
//!                 },
//!             ],
//!         }
//!     }))?;
//!
//!     // Apply the Pod via server-side apply
//!     let params = PatchParams::apply("myapp");
//!     let result = pods.patch("my-pod", &params, &Patch::Apply(&patch)).await?;
//!
//!     // List pods in the configured namespace
//!     for p in pods.list(&ListParams::default()).await? {
//!         println!("found pod {}", p.name());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! For more details, see:
//!
//! - [`Client`](crate::client) for the extensible Kubernetes client
//! - [`Config`](crate::config) for the Kubernetes config abstraction
//! - [`Api`](crate::Api) for the generic api methods available on Kubernetes resources
//! - [k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/) for how to create typed kubernetes objects directly
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]
// Nightly clippy (0.1.64) considers Drop a side effect, see https://github.com/rust-lang/rust-clippy/issues/9608
#![allow(clippy::unnecessary_lazy_evaluations)]

macro_rules! cfg_client {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "client")))]
            #[cfg(feature = "client")]
            $item
        )*
    }
}
macro_rules! cfg_config {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "config")))]
            #[cfg(feature = "config")]
            $item
        )*
    }
}

macro_rules! cfg_error {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(any(feature = "config", feature = "client"))))]
            #[cfg(any(feature = "config", feature = "client"))]
            $item
        )*
    }
}

cfg_client! {
    pub mod api;
    pub mod discovery;
    pub mod client;

    #[doc(inline)]
    pub use api::Api;
    #[doc(inline)]
    pub use client::Client;
    #[doc(inline)]
    pub use discovery::Discovery;
}

cfg_config! {
    pub mod config;
    #[doc(inline)]
    pub use config::Config;
}

cfg_error! {
    pub mod error;
    #[doc(inline)] pub use error::Error;
    /// Convient alias for `Result<T, Error>`
    pub type Result<T, E = Error> = std::result::Result<T, E>;
}

pub use crate::core::{CustomResourceExt, Resource, ResourceExt};
/// Re-exports from kube_core
pub use kube_core as core;

// Tests that require a cluster and the complete feature set
// Can be run with `cargo test -p kube-client --lib features=rustls-tls,ws -- --ignored`
#[cfg(all(feature = "client", feature = "config"))]
#[cfg(test)]
mod test {
    #![allow(unused_imports)]
    use crate::{
        api::{AttachParams, AttachedProcess},
        client::ConfigExt,
        Api, Client, Config, ResourceExt,
    };
    use futures::{StreamExt, TryStreamExt};
    use k8s_openapi::api::core::v1::Pod;
    use kube_core::{
        params::{DeleteParams, Patch},
        response::StatusSummary,
    };
    use serde_json::json;
    use tower::ServiceBuilder;

    // hard disabled test atm due to k3d rustls issues: https://github.com/kube-rs/kube/issues?q=is%3Aopen+is%3Aissue+label%3Arustls
    #[cfg(feature = "when_rustls_works_with_k3d")]
    #[tokio::test]
    #[ignore] // needs cluster (lists pods)
    #[cfg(all(feature = "rustls-tls"))]
    async fn custom_client_rustls_configuration() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::infer().await?;
        let https = config.rustls_https_connector()?;
        let service = ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        let client = Client::new(service, config.default_namespace);
        let pods: Api<Pod> = Api::default_namespaced(client);
        pods.list(&Default::default()).await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (lists pods)
    #[cfg(all(feature = "openssl-tls"))]
    async fn custom_client_openssl_tls_configuration() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::infer().await?;
        let https = config.openssl_https_connector()?;
        let service = ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        let client = Client::new(service, config.default_namespace);
        let pods: Api<Pod> = Api::default_namespaced(client);
        pods.list(&Default::default()).await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (lists api resources)
    #[cfg(all(feature = "discovery"))]
    async fn group_discovery_oneshot() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{core::DynamicObject, discovery};
        let client = Client::try_default().await?;
        let apigroup = discovery::group(&client, "apiregistration.k8s.io").await?;
        let (ar, _caps) = apigroup.recommended_kind("APIService").unwrap();
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
        api.list(&Default::default()).await?;

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (will create and edit a pod)
    async fn pod_can_use_core_apis() -> Result<(), Box<dyn std::error::Error>> {
        use kube::api::{DeleteParams, ListParams, Patch, PatchParams, PostParams, WatchEvent};

        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);

        // create busybox pod that's alive for at most 30s
        let p: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "busybox-kube1",
                "labels": { "app": "kube-rs-test" },
            },
            "spec": {
                "terminationGracePeriodSeconds": 1,
                "restartPolicy": "Never",
                "containers": [{
                  "name": "busybox",
                  "image": "busybox:1.34.1",
                  "command": ["sh", "-c", "sleep 30"],
                }],
            }
        }))?;

        let pp = PostParams::default();
        match pods.create(&pp, &p).await {
            Ok(o) => assert_eq!(p.name_unchecked(), o.name_unchecked()),
            Err(crate::Error::Api(ae)) => assert_eq!(ae.code, 409), // if we failed to clean-up
            Err(e) => return Err(e.into()),                         // any other case if a failure
        }

        // Manual watch-api for it to become ready
        // NB: don't do this; using conditions (see pod_api example) is easier and less error prone
        let lp = ListParams::default()
            .fields(&format!("metadata.name={}", "busybox-kube1"))
            .timeout(15);
        let mut stream = pods.watch(&lp, "0").await?.boxed();
        while let Some(ev) = stream.try_next().await? {
            // can debug format watch event
            let _ = format!("we: {:?}", ev);
            match ev {
                WatchEvent::Modified(o) => {
                    let s = o.status.as_ref().expect("status exists on pod");
                    let phase = s.phase.clone().unwrap_or_default();
                    if phase == "Running" {
                        break;
                    }
                }
                WatchEvent::Error(e) => panic!("watch error: {}", e),
                _ => {}
            }
        }

        // Verify we can get it
        let mut pod = pods.get("busybox-kube1").await?;
        assert_eq!(p.spec.as_ref().unwrap().containers[0].name, "busybox");

        // verify replace with explicit resource version
        // NB: don't do this; use server side apply
        {
            assert!(pod.resource_version().is_some());
            pod.spec.as_mut().unwrap().active_deadline_seconds = Some(5);

            let pp = PostParams::default();
            let patched_pod = pods.replace("busybox-kube1", &pp, &pod).await?;
            assert_eq!(patched_pod.spec.unwrap().active_deadline_seconds, Some(5));
        }

        // Delete it
        let dp = DeleteParams::default();
        pods.delete("busybox-kube1", &dp).await?.map_left(|pdel| {
            assert_eq!(pdel.name_unchecked(), "busybox-kube1");
        });

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (will create and attach to a pod)
    #[cfg(all(feature = "ws"))]
    async fn pod_can_exec_and_write_to_stdin() -> Result<(), Box<dyn std::error::Error>> {
        use crate::api::{DeleteParams, ListParams, Patch, PatchParams, WatchEvent};

        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);

        // create busybox pod that's alive for at most 30s
        let p: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "busybox-kube2",
                "labels": { "app": "kube-rs-test" },
            },
            "spec": {
                "terminationGracePeriodSeconds": 1,
                "restartPolicy": "Never",
                "containers": [{
                  "name": "busybox",
                  "image": "busybox:1.34.1",
                  "command": ["sh", "-c", "sleep 30"],
                }],
            }
        }))?;

        match pods.create(&Default::default(), &p).await {
            Ok(o) => assert_eq!(p.name_unchecked(), o.name_unchecked()),
            Err(crate::Error::Api(ae)) => assert_eq!(ae.code, 409), // if we failed to clean-up
            Err(e) => return Err(e.into()),                         // any other case if a failure
        }

        // Manual watch-api for it to become ready
        // NB: don't do this; using conditions (see pod_api example) is easier and less error prone
        let lp = ListParams::default()
            .fields(&format!("metadata.name={}", "busybox-kube2"))
            .timeout(15);
        let mut stream = pods.watch(&lp, "0").await?.boxed();
        while let Some(ev) = stream.try_next().await? {
            match ev {
                WatchEvent::Modified(o) => {
                    let s = o.status.as_ref().expect("status exists on pod");
                    let phase = s.phase.clone().unwrap_or_default();
                    if phase == "Running" {
                        break;
                    }
                }
                WatchEvent::Error(e) => panic!("watch error: {}", e),
                _ => {}
            }
        }

        // Verify exec works and we can get the output
        {
            let mut attached = pods
                .exec(
                    "busybox-kube2",
                    vec!["sh", "-c", "for i in $(seq 1 3); do echo $i; done"],
                    &AttachParams::default().stderr(false),
                )
                .await?;
            let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
            let out = stdout
                .filter_map(|r| async { r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok()) })
                .collect::<Vec<_>>()
                .await
                .join("");
            attached.join().await.unwrap();
            assert_eq!(out.lines().count(), 3);
            assert_eq!(out, "1\n2\n3\n");
        }

        // Verify we can write to Stdin
        {
            use tokio::io::AsyncWriteExt;
            let mut attached = pods
                .exec(
                    "busybox-kube2",
                    vec!["sh"],
                    &AttachParams::default().stdin(true).stderr(false),
                )
                .await?;
            let mut stdin_writer = attached.stdin().unwrap();
            let mut stdout_stream = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
            let next_stdout = stdout_stream.next();
            stdin_writer.write_all(b"echo test string 1\n").await?;
            let stdout = String::from_utf8(next_stdout.await.unwrap().unwrap().to_vec()).unwrap();
            println!("{}", stdout);
            assert_eq!(stdout, "test string 1\n");

            // AttachedProcess resolves with status object.
            // Send `exit 1` to get a failure status.
            stdin_writer.write_all(b"exit 1\n").await?;
            let status = attached.take_status().unwrap();
            if let Some(status) = status.await {
                println!("{:?}", status);
                assert_eq!(status.status, Some("Failure".to_owned()));
                assert_eq!(status.reason, Some("NonZeroExitCode".to_owned()));
            }
        }

        // Delete it
        let dp = DeleteParams::default();
        pods.delete("busybox-kube2", &dp).await?.map_left(|pdel| {
            assert_eq!(pdel.name_unchecked(), "busybox-kube2");
        });

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (will create and tail logs from a pod)
    async fn can_get_pod_logs_and_evict() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            api::{DeleteParams, EvictParams, ListParams, Patch, PatchParams, WatchEvent},
            core::subresource::LogParams,
        };

        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);

        // create busybox pod that's alive for at most 30s
        let p: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "busybox-kube3",
                "labels": { "app": "kube-rs-test" },
            },
            "spec": {
                "terminationGracePeriodSeconds": 1,
                "restartPolicy": "Never",
                "containers": [{
                  "name": "busybox",
                  "image": "busybox:1.34.1",
                  "command": ["sh", "-c", "for i in $(seq 1 5); do echo kube $i; sleep 0.1; done"],
                }],
            }
        }))?;

        match pods.create(&Default::default(), &p).await {
            Ok(o) => assert_eq!(p.name_unchecked(), o.name_unchecked()),
            Err(crate::Error::Api(ae)) => assert_eq!(ae.code, 409), // if we failed to clean-up
            Err(e) => return Err(e.into()),                         // any other case if a failure
        }

        // Manual watch-api for it to become ready
        // NB: don't do this; using conditions (see pod_api example) is easier and less error prone
        let lp = ListParams::default()
            .fields(&format!("metadata.name={}", "busybox-kube3"))
            .timeout(15);
        let mut stream = pods.watch(&lp, "0").await?.boxed();
        while let Some(ev) = stream.try_next().await? {
            match ev {
                WatchEvent::Modified(o) => {
                    let s = o.status.as_ref().expect("status exists on pod");
                    let phase = s.phase.clone().unwrap_or_default();
                    if phase == "Running" {
                        break;
                    }
                }
                WatchEvent::Error(e) => panic!("watch error: {}", e),
                _ => {}
            }
        }

        // Get current list of logs
        let lp = LogParams {
            follow: true,
            ..LogParams::default()
        };
        let mut logs_stream = pods.log_stream("busybox-kube3", &lp).await?.boxed();
        let log_line = logs_stream.try_next().await?.unwrap();
        assert_eq!(log_line, "kube 1\n");

        // wait for container to finish
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let all_logs = pods.logs("busybox-kube3", &Default::default()).await?;
        assert_eq!(all_logs, "kube 1\nkube 2\nkube 3\nkube 4\nkube 5\n");

        // remaining logs should have been buffered internally
        assert_eq!(logs_stream.try_next().await?.unwrap(), "kube 2\n");
        assert_eq!(logs_stream.try_next().await?.unwrap(), "kube 3\n");
        assert_eq!(logs_stream.try_next().await?.unwrap(), "kube 4\n");
        assert_eq!(logs_stream.try_next().await?.unwrap(), "kube 5\n");

        // evict the pod
        let ep = EvictParams::default();
        let eres = pods.evict("busybox-kube3", &ep).await?;
        assert_eq!(eres.code, 201); // created
        assert!(eres.is_success());

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (will create a CertificateSigningRequest)
    async fn csr_can_be_approved() -> Result<(), Box<dyn std::error::Error>> {
        use crate::api::PostParams;
        use k8s_openapi::api::certificates::v1::{
            CertificateSigningRequest, CertificateSigningRequestCondition, CertificateSigningRequestStatus,
        };

        let csr_name = "fake";
        let dummy_csr: CertificateSigningRequest = serde_json::from_value(json!({
            "apiVersion": "certificates.k8s.io/v1",
            "kind": "CertificateSigningRequest",
            "metadata": { "name": csr_name },
            "spec": {
                "request": "LS0tLS1CRUdJTiBDRVJUSUZJQ0FURSBSRVFVRVNULS0tLS0KTUlJQ1ZqQ0NBVDRDQVFBd0VURVBNQTBHQTFVRUF3d0dZVzVuWld4aE1JSUJJakFOQmdrcWhraUc5dzBCQVFFRgpBQU9DQVE4QU1JSUJDZ0tDQVFFQTByczhJTHRHdTYxakx2dHhWTTJSVlRWMDNHWlJTWWw0dWluVWo4RElaWjBOCnR2MUZtRVFSd3VoaUZsOFEzcWl0Qm0wMUFSMkNJVXBGd2ZzSjZ4MXF3ckJzVkhZbGlBNVhwRVpZM3ExcGswSDQKM3Z3aGJlK1o2MVNrVHF5SVBYUUwrTWM5T1Nsbm0xb0R2N0NtSkZNMUlMRVI3QTVGZnZKOEdFRjJ6dHBoaUlFMwpub1dtdHNZb3JuT2wzc2lHQ2ZGZzR4Zmd4eW8ybmlneFNVekl1bXNnVm9PM2ttT0x1RVF6cXpkakJ3TFJXbWlECklmMXBMWnoyalVnald4UkhCM1gyWnVVV1d1T09PZnpXM01LaE8ybHEvZi9DdS8wYk83c0x0MCt3U2ZMSU91TFcKcW90blZtRmxMMytqTy82WDNDKzBERHk5aUtwbXJjVDBnWGZLemE1dHJRSURBUUFCb0FBd0RRWUpLb1pJaHZjTgpBUUVMQlFBRGdnRUJBR05WdmVIOGR4ZzNvK21VeVRkbmFjVmQ1N24zSkExdnZEU1JWREkyQTZ1eXN3ZFp1L1BVCkkwZXpZWFV0RVNnSk1IRmQycVVNMjNuNVJsSXJ3R0xuUXFISUh5VStWWHhsdnZsRnpNOVpEWllSTmU3QlJvYXgKQVlEdUI5STZXT3FYbkFvczFqRmxNUG5NbFpqdU5kSGxpT1BjTU1oNndLaTZzZFhpVStHYTJ2RUVLY01jSVUyRgpvU2djUWdMYTk0aEpacGk3ZnNMdm1OQUxoT045UHdNMGM1dVJVejV4T0dGMUtCbWRSeEgvbUNOS2JKYjFRQm1HCkkwYitEUEdaTktXTU0xMzhIQXdoV0tkNjVoVHdYOWl4V3ZHMkh4TG1WQzg0L1BHT0tWQW9FNkpsYWFHdTlQVmkKdjlOSjVaZlZrcXdCd0hKbzZXdk9xVlA3SVFjZmg3d0drWm89Ci0tLS0tRU5EIENFUlRJRklDQVRFIFJFUVVFU1QtLS0tLQo=",
                "signerName": "kubernetes.io/kube-apiserver-client",
                "expirationSeconds": 86400,
                "usages": ["client auth"]
            }
        }))?;

        let client = Client::try_default().await?;
        let csr: Api<CertificateSigningRequest> = Api::all(client.clone());
        assert!(csr.create(&PostParams::default(), &dummy_csr).await.is_ok());

        // Patch the approval and approve the CSR
        let approval_type = "ApprovedFake";
        let csr_status: CertificateSigningRequestStatus = CertificateSigningRequestStatus {
            certificate: None,
            conditions: Some(vec![CertificateSigningRequestCondition {
                type_: approval_type.to_string(),
                last_update_time: None,
                last_transition_time: None,
                message: Some(format!("{} {}", approval_type, "by kube-rs client")),
                reason: Some("kube-rsClient".to_string()),
                status: "True".to_string(),
            }]),
        };
        let csr_status_patch = Patch::Merge(serde_json::json!({ "status": csr_status }));
        let _ = csr
            .patch_approval(csr_name, &Default::default(), &csr_status_patch)
            .await?;
        let csr_after_approval = csr.get_approval(csr_name).await?;

        assert_eq!(
            csr_after_approval
                .status
                .as_ref()
                .unwrap()
                .conditions
                .as_ref()
                .unwrap()[0]
                .type_,
            approval_type.to_string()
        );
        csr.delete(csr_name, &DeleteParams::default()).await?;
        Ok(())
    }
}
