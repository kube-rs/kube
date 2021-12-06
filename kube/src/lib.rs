//! Kube is an umbrella-crate for interacting with [Kubernetes](http://kubernetes.io) in Rust.
//!
//! # Overview
//!
//! Kube contains a Kubernetes client, a controller runtime, a custom resource derive, and various tooling
//! required for building applications or controllers that interact with Kubernetes.
//!
//! The main modules are:
//!
//! - [`client`](crate::client) with the Kubernetes [`Client`](crate::Client) and its layers
//! - [`config`](crate::config) for cluster [`Config`](crate::Config)
//! - [`api`](crate::api) with the generic Kubernetes [`Api`](crate::Api)
//! - [`derive`](kube_derive) with the [`CustomResource`](crate::CustomResource) derive for building controllers types
//! - [`runtime`](crate::runtime) with a [`Controller`](crate::runtime::Controller) / [`watcher`](crate::runtime::watcher()) / [`reflector`](crate::runtime::reflector::reflector) / [`Store`](crate::runtime::reflector::Store)
//! - [`core`](crate::core) with generics from `apimachinery`
//!
//! You can use each of these as you need with the help of the [exported features](https://github.com/kube-rs/kube-rs/blob/master/kube/Cargo.toml#L18).
//!
//! # Using the Client
//! ```no_run
//! use futures::{StreamExt, TryStreamExt};
//! use kube::{Client, api::{Api, ResourceExt, ListParams, PostParams}};
//! use k8s_openapi::api::core::v1::Pod;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Infer the runtime environment and try to create a Kubernetes Client
//!     let client = Client::try_default().await?;
//!
//!     // Read pods in the configured namespace into the typed interface from k8s-openapi
//!     let pods: Api<Pod> = Api::default_namespaced(client);
//!     for p in pods.list(&ListParams::default()).await? {
//!         println!("found pod {}", p.name());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! For details, see:
//!
//! - [`Client`](crate::client) for the extensible Kubernetes client
//! - [`Api`](crate::Api) for the generic api methods available on Kubernetes resources
//! - [k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/) for documentation about the generated Kubernetes types
//!
//! # Using the Runtime with the Derive macro
//!
//! ```no_run
//! use schemars::JsonSchema;
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//! use validator::Validate;
//! use futures::{StreamExt, TryStreamExt};
//! use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
//! use kube::{
//!     api::{Api, DeleteParams, ListParams, PatchParams, Patch, ResourceExt},
//!     core::CustomResourceExt,
//!     Client, CustomResource,
//!     runtime::{watcher, utils::try_flatten_applied, wait::{conditions, await_condition}},
//! };
//!
//! // Our custom resource
//! #[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
//! #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
//! pub struct FooSpec {
//!     info: String,
//!     #[validate(length(min = 3))]
//!     name: String,
//!     replicas: i32,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::try_default().await?;
//!     let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
//!
//!     // Apply the CRD so users can create Foo instances in Kubernetes
//!     crds.patch("foos.clux.dev",
//!         &PatchParams::apply("my_manager"),
//!         &Patch::Apply(Foo::crd())
//!     ).await?;
//!
//!     // Wait for the CRD to be ready
//!     tokio::time::timeout(
//!         std::time::Duration::from_secs(10),
//!         await_condition(crds, "foos.clux.dev", conditions::is_crd_established())
//!     ).await?;
//!
//!     // Watch for changes to foos in the configured namespace
//!     let foos: Api<Foo> = Api::default_namespaced(client.clone());
//!     let lp = ListParams::default();
//!     let mut apply_stream = try_flatten_applied(watcher(foos, lp)).boxed();
//!     while let Some(f) = apply_stream.try_next().await? {
//!         println!("saw apply to {}", f.name());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! For details, see:
//!
//! - [`CustomResource`](crate::CustomResource) for documentation how to configure custom resources
//! - [`runtime::watcher`](crate::runtime::watcher()) for how to long-running watches work and why you want to use this over [`Api::watch`](crate::Api::watch)
//! - [`runtime`](crate::runtime) for abstractions that help with more complicated Kubernetes application
//!
//! # Examples
//! A large list of complete, runnable examples with explainations are available in the [examples folder](https://github.com/kube-rs/kube-rs/tree/master/examples).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

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
    pub use kube_client::api;
    pub use kube_client::discovery;
    pub use kube_client::client;

    #[doc(inline)]
    pub use api::Api;
    #[doc(inline)]
    pub use client::Client;
    #[doc(inline)]
    pub use discovery::Discovery;
}

cfg_config! {
    pub use kube_client::config;
    #[doc(inline)]
    pub use config::Config;
}

cfg_error! {
    pub use kube_client::error;
    #[doc(inline)] pub use error::Error;
    /// Convient alias for `Result<T, Error>`
    pub type Result<T, E = Error> = std::result::Result<T, E>;
}

/// Re-exports from [`kube-derive`](kube_derive)
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use kube_derive::CustomResource;

/// Re-exports from [`kube-runtime`](kube_runtime)
#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
#[doc(inline)]
pub use kube_runtime as runtime;

pub use crate::core::{CustomResourceExt, Resource, ResourceExt};
/// Re-exports from [`kube_core`](kube_core)
#[doc(inline)]
pub use kube_core as core;

// Tests that require a cluster and the complete feature set
// Can be run with `cargo test -p kube --lib --features=runtime,derive -- --ignored`
#[cfg(test)]
#[cfg(all(feature = "derive", feature = "client"))]
mod test {
    use crate::{Api, Client, CustomResourceExt, ResourceExt};
    use kube_derive::CustomResource;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
    #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
    #[kube(status = "FooStatus")]
    #[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
    #[kube(crates(kube_core = "crate::core"))] // for dev-dep test structure
    pub struct FooSpec {
        name: String,
        info: Option<String>,
        replicas: isize,
    }

    #[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
    pub struct FooStatus {
        is_bad: bool,
        replicas: isize,
    }

    #[tokio::test]
    #[ignore] // needs kubeconfig
    async fn custom_resource_generates_correct_core_structs() {
        use crate::core::{ApiResource, DynamicObject, GroupVersionKind};
        let client = Client::try_default().await.unwrap();

        let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
        let api_resource = ApiResource::from_gvk(&gvk);
        let a1: Api<DynamicObject> = Api::namespaced_with(client.clone(), "myns", &api_resource);
        let a2: Api<Foo> = Api::namespaced(client, "myns");

        // make sure they return the same url_path through their impls
        assert_eq!(a1.resource_url(), a2.resource_url());
    }

    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    #[tokio::test]
    #[ignore] // needs cluster (creates + patches foo crd)
    #[cfg(all(feature = "derive", feature = "runtime"))]
    async fn derived_resource_queriable_and_has_subresources() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            core::params::{DeleteParams, Patch, PatchParams},
            runtime::wait::{await_condition, conditions},
        };
        use serde_json::json;
        let client = Client::try_default().await?;
        let ssapply = PatchParams::apply("kube").force();
        let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
        // Server-side apply CRD
        crds.patch("foos.clux.dev", &ssapply, &Patch::Apply(Foo::crd()))
            .await?;
        // Wait for it to be ready:
        let establish = await_condition(crds, "foos.clux.dev", conditions::is_crd_established());
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), establish).await?;
        // Use it
        let foos: Api<Foo> = Api::default_namespaced(client.clone());
        // Apply from generated struct
        {
            let foo = Foo::new("baz", FooSpec {
                name: "baz".into(),
                info: Some("old baz".into()),
                replicas: 1,
            });
            let o = foos.patch("baz", &ssapply, &Patch::Apply(&foo)).await?;
            assert_eq!(o.spec.name, "baz");
        }
        // Apply from partial json!
        {
            let patch = json!({
                "apiVersion": "clux.dev/v1",
                "kind": "Foo",
                "spec": {
                    "name": "foo",
                    "replicas": 2
                }
            });
            let o = foos.patch("baz", &ssapply, &Patch::Apply(patch)).await?;
            assert_eq!(o.spec.replicas, 2);
        }
        // check subresource
        {
            assert_eq!(foos.get_scale("baz").await?.spec.unwrap().replicas, Some(2));
            assert!(foos.get_status("baz").await?.status.is_none()); // nothing has set this
        }
        // set status subresource
        {
            let fs = serde_json::json!({"status": FooStatus { is_bad: false, replicas: 1 }});
            let o = foos
                .patch_status("baz", &Default::default(), &Patch::Merge(&fs))
                .await?;
            assert!(o.status.is_some());
        }
        // set scale subresource
        {
            let fs = serde_json::json!({"spec": { "replicas": 3 }});
            let o = foos
                .patch_scale("baz", &Default::default(), &Patch::Merge(&fs))
                .await?;
            assert_eq!(o.status.unwrap().replicas, 1); // something needs to set the status for this
            assert_eq!(o.spec.unwrap().replicas.unwrap(), 3); // what we asked for got updated
        }

        // cleanup
        foos.delete("baz", &DeleteParams::default()).await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (lists pods)
    async fn custom_objects_are_usable() -> Result<(), Box<dyn std::error::Error>> {
        use crate::core::{ApiResource, NotUsed, Object};
        use k8s_openapi::api::core::v1::Pod;
        #[derive(Clone, Deserialize, Debug)]
        struct PodSpecSimple {
            containers: Vec<ContainerSimple>,
        }
        #[derive(Clone, Deserialize, Debug)]
        struct ContainerSimple {
            image: String,
        }
        type PodSimple = Object<PodSpecSimple, NotUsed>;
        // use known type information from pod (can also use discovery for this)
        let ar = ApiResource::erase::<Pod>(&());

        let client = Client::try_default().await?;
        let api: Api<PodSimple> = Api::default_namespaced_with(client, &ar);
        for p in api.list(&Default::default()).await? {
            // run loop to cover ObjectList::iter
            println!("found pod {} with containers: {:?}", p.name(), p.spec.containers);
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (fetches api resources, and lists cr)
    #[cfg(all(feature = "derive"))]
    async fn derived_resources_discoverable() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            core::{DynamicObject, GroupVersion, GroupVersionKind},
            discovery,
        };
        let client = Client::try_default().await?;
        let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
        let gv = GroupVersion::gv("clux.dev", "v1");

        // discover by both (recommended kind on groupversion) and (pinned gvk) and they should equal
        let apigroup = discovery::oneshot::pinned_group(&client, &gv).await?;
        let (ar1, caps1) = apigroup.recommended_kind("Foo").unwrap();
        let (ar2, caps2) = discovery::pinned_kind(&client, &gvk).await?;
        assert_eq!(caps1.operations.len(), caps2.operations.len());
        assert_eq!(ar1, ar2);

        let api = Api::<DynamicObject>::all_with(client, &ar2);
        api.list(&Default::default()).await?;

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (fetches api resources, and lists all)
    async fn resources_discoverable() -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            core::{DynamicObject, GroupVersionKind},
            discovery::{verbs, Discovery, Scope},
        };
        let client = Client::try_default().await?;

        let discovery = Discovery::new(client.clone())
            .exclude(&[""]) // skip core resources
            .run()
            .await?;
        // check our custom resource first by resolving within groups
        {
            assert!(discovery.has_group("clux.dev"));
            let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
            let (ar, _caps) = discovery.resolve_gvk(&gvk).unwrap();
            assert_eq!(ar.group, gvk.group);
            assert_eq!(ar.version, gvk.version);
            assert_eq!(ar.kind, gvk.kind);
        }
        // check all non-excluded groups that are iterable
        for group in discovery.groups() {
            for (ar, caps) in group.recommended_resources() {
                if !caps.supports_operation(verbs::LIST) {
                    continue;
                }
                let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
                    Api::default_namespaced_with(client.clone(), &ar)
                } else {
                    Api::all_with(client.clone(), &ar)
                };
                api.list(&Default::default()).await?;
            }
        }
        Ok(())
    }
}
