//! Trigger mappers that turn streams of objects into streams of [`ReconcileRequest`]s
//!
//! Every relation (self, owns, watches) is a mapping from an object in a watched stream
//! to zero or more reconcile requests for the controlled kind, built on top of [`trigger_with`].
//!
//! Input streams can yield either owned objects (`K`) or shared objects (`Arc<K>`), since
//! both implement [`Borrow<K>`](std::borrow::Borrow).

use super::{ReconcileReason, ReconcileRequest};
use crate::reflector::ObjectRef;
use futures::{Stream, TryStream, TryStreamExt, stream};
use kube_client::Resource;
use std::borrow::Borrow;

/// Helper for building custom trigger filters, see the implementations of [`trigger_self`] and [`trigger_owners`] for some examples.
pub fn trigger_with<T, K, I, S>(
    stream: S,
    mapper: impl Fn(T) -> I,
) -> impl Stream<Item = Result<ReconcileRequest<K>, S::Error>>
where
    S: TryStream<Ok = T>,
    I: IntoIterator,
    I::Item: Into<ReconcileRequest<K>>,
    K: Resource,
{
    stream
        .map_ok(move |obj| stream::iter(mapper(obj).into_iter().map(Into::into).map(Ok)))
        .try_flatten()
}

/// Enqueues the object itself for reconciliation
///
/// The stream can yield `K` or `Arc<K>`.
pub fn trigger_self<K, S>(
    stream: S,
    dyntype: K::DynamicType,
) -> impl Stream<Item = Result<ReconcileRequest<K>, S::Error>>
where
    S: TryStream,
    S::Ok: Borrow<K>,
    K: Resource,
    K::DynamicType: Clone,
{
    trigger_with(stream, move |obj: S::Ok| {
        Some(ReconcileRequest {
            obj_ref: ObjectRef::from_obj_with(obj.borrow(), dyntype.clone()),
            reason: ReconcileReason::ObjectUpdated,
        })
    })
}

/// Enqueues any mapper returned `K` types for reconciliation
///
/// The stream can yield `O` or `Arc<O>`, and the mapper receives the item as yielded.
pub(crate) fn trigger_others<S, O, K, I>(
    stream: S,
    mapper: impl Fn(S::Ok) -> I + Sync + Send + 'static,
    dyntype: O::DynamicType,
) -> impl Stream<Item = Result<ReconcileRequest<K>, S::Error>>
where
    // Input stream has items as some Resource (via Controller::watches)
    S: TryStream,
    S::Ok: Borrow<O>,
    O: Resource,
    O::DynamicType: Clone,
    // Output stream is requests for the root type K
    K: Resource,
    K::DynamicType: Clone,
    // but the mapper can produce many of them
    I: 'static + IntoIterator<Item = ObjectRef<K>>,
    I::IntoIter: Send,
{
    trigger_with(stream, move |obj: S::Ok| {
        let watch_ref = ObjectRef::from_obj_with(obj.borrow(), dyntype.clone()).erase();
        mapper(obj)
            .into_iter()
            .map(move |mapped_obj_ref| ReconcileRequest {
                obj_ref: mapped_obj_ref,
                reason: ReconcileReason::RelatedObjectUpdated {
                    obj_ref: Box::new(watch_ref.clone()),
                },
            })
    })
}

/// Enqueues any owners of type `KOwner` for reconciliation
///
/// The stream can yield `K` or `Arc<K>`.
pub fn trigger_owners<KOwner, K, S>(
    stream: S,
    owner_type: KOwner::DynamicType,
    child_type: K::DynamicType,
) -> impl Stream<Item = Result<ReconcileRequest<KOwner>, S::Error>>
where
    S: TryStream,
    S::Ok: Borrow<K>,
    K: Resource,
    K::DynamicType: Clone,
    KOwner: Resource,
    KOwner::DynamicType: Clone,
{
    let mapper = move |obj: S::Ok| {
        // only clone the two fields we need rather than the whole ObjectMeta
        let meta = obj.borrow().meta();
        let ns = meta.namespace.clone();
        let owner_type = owner_type.clone();
        meta.owner_references
            .clone()
            .into_iter()
            .flatten()
            .filter_map(move |owner| ObjectRef::from_owner_ref(ns.as_deref(), &owner, owner_type.clone()))
    };
    trigger_others::<S, K, KOwner, _>(stream, mapper, child_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, TryStreamExt};
    use k8s_openapi::{
        api::{
            apps::v1::{Deployment, ReplicaSet},
            core::v1::{ConfigMap, Pod},
        },
        apimachinery::pkg::apis::meta::v1::OwnerReference,
    };
    use kube_client::core::ObjectMeta;
    use std::{convert::Infallible, sync::Arc};

    fn owner_ref<K: Resource<DynamicType = ()>>(name: &str) -> OwnerReference {
        OwnerReference {
            api_version: K::api_version(&()).into(),
            kind: K::kind(&()).into(),
            name: name.into(),
            uid: format!("{name}-uid"),
            ..Default::default()
        }
    }

    fn pod(name: &str, owners: Vec<OwnerReference>) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(name.into()),
                namespace: Some("ns".into()),
                owner_references: Some(owners),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ok<T>(items: Vec<T>) -> impl Stream<Item = Result<T, Infallible>> {
        stream::iter(items).map(Ok)
    }

    async fn collect<K: Resource>(
        s: impl Stream<Item = Result<ReconcileRequest<K>, Infallible>>,
    ) -> Vec<ReconcileRequest<K>> {
        s.try_collect().await.unwrap()
    }

    fn assert_related_to(req: &ReconcileRequest<impl Resource>, expected: &ObjectRef<Pod>) {
        match &req.reason {
            ReconcileReason::RelatedObjectUpdated { obj_ref } => {
                assert_eq!(**obj_ref, expected.clone().erase())
            }
            reason => panic!("unexpected reason: {reason}"),
        }
    }

    #[tokio::test]
    async fn trigger_self_maps_owned_and_shared_objects() {
        let p = pod("a", vec![]);
        let expected = ObjectRef::from_obj(&p);

        for reqs in [
            collect(trigger_self::<Pod, _>(ok(vec![p.clone()]), ())).await,
            collect(trigger_self::<Pod, _>(ok(vec![Arc::new(p.clone())]), ())).await,
        ] {
            assert_eq!(reqs.len(), 1);
            assert_eq!(reqs[0].obj_ref, expected);
            assert!(matches!(reqs[0].reason, ReconcileReason::ObjectUpdated));
        }
    }

    #[tokio::test]
    async fn trigger_owners_only_enqueues_matching_owner_kind() {
        let p = pod("a", vec![
            owner_ref::<ReplicaSet>("rs"),
            owner_ref::<Deployment>("ignored"),
        ]);
        let orphan = pod("b", vec![]);
        let child_ref = ObjectRef::from_obj(&p);

        for reqs in [
            collect(trigger_owners::<ReplicaSet, Pod, _>(
                ok(vec![p.clone(), orphan.clone()]),
                (),
                (),
            ))
            .await,
            collect(trigger_owners::<ReplicaSet, Pod, _>(
                ok(vec![Arc::new(p.clone()), Arc::new(orphan.clone())]),
                (),
                (),
            ))
            .await,
        ] {
            assert_eq!(reqs.len(), 1);
            let req = &reqs[0];
            assert_eq!(req.obj_ref, ObjectRef::new("rs").within("ns"));
            assert_eq!(req.obj_ref.extra.uid.as_deref(), Some("rs-uid"));
            assert_related_to(req, &child_ref);
        }
    }

    #[tokio::test]
    async fn trigger_others_enqueues_every_mapped_object() {
        let p = pod("a", vec![]);
        let pod_ref = ObjectRef::from_obj(&p);
        let mapper = |pod: Arc<Pod>| {
            let ns = pod.metadata.namespace.clone().unwrap();
            ["cm1", "cm2"].map(|name| ObjectRef::<ConfigMap>::new(name).within(&ns))
        };

        let reqs = collect(trigger_others::<_, Pod, ConfigMap, _>(
            ok(vec![Arc::new(p)]),
            mapper,
            (),
        ))
        .await;

        let names: Vec<_> = reqs.iter().map(|r| r.obj_ref.name.as_str()).collect();
        assert_eq!(names, ["cm1", "cm2"]);
        for req in &reqs {
            assert_eq!(req.obj_ref.namespace.as_deref(), Some("ns"));
            assert_related_to(req, &pod_ref);
        }
    }

    #[tokio::test]
    async fn trigger_with_passes_errors_through() {
        let input = stream::iter(vec![Err("boom"), Ok(pod("a", vec![]))]);
        let out: Vec<_> = trigger_self::<Pod, _>(input, ()).collect().await;

        assert!(matches!(out[0], Err("boom")));
        assert_eq!(out[1].as_ref().unwrap().obj_ref.name, "a");
    }
}
