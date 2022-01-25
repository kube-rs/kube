//! Acquire and hold a lease (advisory lock)

use k8s_openapi::{
    api::coordination::v1::{Lease as KubeLease, LeaseSpec as KubeLeaseSpec},
    apimachinery::pkg::apis::meta::v1::MicroTime,
    chrono::{Local, Utc}
};

use kube_client::api::{ ObjectMeta, Patch, PatchParams, PostParams };

use std::time::Duration;
use tokio::{sync::oneshot::Sender, task::JoinHandle};
use kube_client::Api;
use kube_client::error::ErrorResponse;

const LEASE_DURATION_SECONDS: u64 = 5;

/// This implementation provides a guard for routinely renewing an Lease object until dropped. Once
/// dropped the spawned renewal task is allowed to complete. This implementation also allows callers
/// to poll the availability of a Lease, and also perform a blocking call to acquire a lease.
/// Acquiring an existing Lease is done with a compare and swap, relying on the Kubernetes API to
/// ensure excusive ownership of Lease.
///
/// Leases are "acquired", then periodically "renewed", and they track the "owner" of the lease. The renewal
/// frequency is not configurable but leases are released as soon as the guard is dropped.
/// When a lease is no longer needed, all the properties are nullified so other processes can take over.
///
/// A Lease is a great tool for signaling a process is underway -- this ensures cooperating software should
/// wait before continuing. Leases are named and namespaced objects.
///
/// Future work should attach Events to a Lease so `kubernetes describe` can make a useful history of the
/// lease.
///
/// How to use a lease:
///
///
///
pub struct Lease {
    join_handle: JoinHandle<()>,
    sender: Sender<()>,
}

impl Lease {
    pub fn lease_name() -> &'static str {
        "cousteau"
    }

    pub fn lease_duration() -> u64 {
        LEASE_DURATION_SECONDS
    }

    pub async fn available(
        kube_api_client: kube_client::Client,
        ns: &str,
        lease_name: &str,
    ) -> Result<bool, String> {
        let lease_client: Api<KubeLease> =
            kube_client::api::Api::namespaced(kube_api_client.clone(), ns);

        let get_lease = lease_client.get(lease_name).await;

        match get_lease {
            Err(kube_client::Error::Api(ErrorResponse { code: 404, .. })) => Ok(true),
            Err(err) => Err(format!("{:#?}", err)),
            Ok(lease) => Ok(Self::lease_expired(&lease)),
        }
    }

    pub async fn acquire_or_create(
        kube_api_client: kube_client::Client,
        ns: &str,
        lease_name: &str,
        identity: &str,
    ) -> Result<Lease, ()> {
        // let lease_client = self.kube_api_client
        let lease_client: Api<KubeLease> =
            kube_client::api::Api::namespaced(kube_api_client.clone(), ns);

        // check for lease
        let lease = loop {
            let get_lease = lease_client.get(lease_name).await;

            if let Err(kube_client::Error::Api(ErrorResponse { code: 404, .. })) = get_lease {
                tracing::trace!("lease does not exist, instantiating with defaults");
                let lease = lease_client
                    .create(
                        &PostParams::default(),
                        &KubeLease {
                            metadata: ObjectMeta {
                                namespace: Some(ns.to_string()),
                                name: Some(lease_name.to_string()),
                                ..Default::default()
                            },
                            spec: Some(KubeLeaseSpec {
                                acquire_time: Some(Self::now()),
                                lease_duration_seconds: Some(LEASE_DURATION_SECONDS as i32),
                                holder_identity: Some(identity.to_string()),
                                lease_transitions: Some(1),
                                ..Default::default()
                            }),
                        },
                    )
                    .await
                    .unwrap();
                break lease;
            } else if let Ok(mut lease) = get_lease {
                if Self::lease_expired(&lease) {
                    tracing::trace!("the lease expired, taking ownership");
                    lease.metadata.managed_fields = None;

                    let spec = lease.spec.as_mut().unwrap();

                    if spec.lease_transitions.is_none() {
                        spec.lease_transitions = Some(0);
                    }
                    spec.lease_transitions.as_mut().map(|lt| *lt = *lt + 1);
                    spec.acquire_time = Some(Self::now());
                    spec.renew_time = None;
                    spec.lease_duration_seconds = Some(LEASE_DURATION_SECONDS as i32);
                    spec.holder_identity = Some(identity.to_string());

                    lease = lease_client
                        .patch(
                            lease_name,
                            &PatchParams::apply("cousteau").force(),
                            &Patch::Apply(serde_json::to_vec(&lease).unwrap()),
                        )
                        .await
                        .unwrap();
                    break lease;
                } else {
                    let wait_time = match lease.spec {
                        Some(KubeLeaseSpec {
                                 lease_duration_seconds: Some(lds),
                                 ..
                             }) => lds as u64,
                        _ => LEASE_DURATION_SECONDS,
                    };
                    tracing::trace!(
                        "lease is not ready, let's wait {} seconds and try again",
                        wait_time
                    );
                    tokio::time::sleep(Duration::from_secs(wait_time)).await;
                    continue;
                }
            } else {
                panic!("what in the {:#?}", get_lease);
            };
        };

        let (sender, mut recv) = tokio::sync::oneshot::channel();

        let renew_client = lease_client.clone();
        let mut renew_resource_version = lease.metadata.resource_version.clone();
        let renew_object_name = lease_name.to_string();
        let renew_lease_duration_seconds =
            lease.spec.as_ref().unwrap().lease_duration_seconds.unwrap();

        let join_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                renew_lease_duration_seconds as u64,
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        tracing::trace!("interval tick fired, good time to renew stuff");
                        let patch_params = PatchParams::apply("cousteau");
                        let patch = serde_json::json!({
                            "apiVersion": "coordination.k8s.io/v1",
                            "kind": "Lease",
                            "metadata": {
                                "resourceVersion": renew_resource_version,
                                "name": renew_object_name
                            },
                            "spec": {
                                "renewTime": Self::now(),
                            }
                        });
                        let patch_res = renew_client.patch(&renew_object_name, &patch_params, &Patch::Apply(patch)).await.unwrap();
                        renew_resource_version = patch_res.metadata.resource_version;
                    }
                    _ = &mut recv => {
                        tracing::trace!("receiver woke up");
                        break
                    }
                }
            }
            tracing::trace!("all done looping, zeroing out the lease");

            let patch_params = PatchParams::apply("cousteau");
            let patch = serde_json::json!({
                "apiVersion": "coordination.k8s.io/v1",
                "kind": "Lease",
                "metadata": {
                    "resourceVersion": renew_resource_version,
                    "name": renew_object_name
                },
                "spec": {
                    "renewTime": Option::<()>::None,
                    "acquireTime": Option::<()>::None,
                    "holderIdentity": Option::<()>::None
                }
            });
            renew_client
                .patch(&renew_object_name, &patch_params, &Patch::Apply(patch))
                .await
                .unwrap();

            tracing::trace!("all done with the lease");
        });

        return Ok(Lease {
            join_handle,
            sender,
        });
    }

    fn now() -> MicroTime {
        let local_now = Local::now();
        MicroTime(local_now.with_timezone(&Utc))
    }

    fn lease_expired(lease: &KubeLease) -> bool {
        let KubeLeaseSpec {
            acquire_time,
            renew_time,
            lease_duration_seconds,
            ..
        } = lease.spec.as_ref().unwrap();

        let local_now = Local::now();
        let utc_now = local_now.with_timezone(&Utc);

        let lease_duration = chrono::Duration::seconds(
            *lease_duration_seconds
                .as_ref()
                .unwrap_or(&(LEASE_DURATION_SECONDS as i32)) as i64,
        );
        if let Some(MicroTime(time)) = renew_time {
            let renew_expire = time.checked_add_signed(lease_duration).unwrap();
            return utc_now.gt(&renew_expire);
        } else if let Some(MicroTime(time)) = acquire_time {
            let acquire_expire = time.checked_add_signed(lease_duration).unwrap();
            return utc_now.gt(&acquire_expire);
        }

        return true;
    }

    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.sender.send(()).unwrap();
        self.join_handle.await
    }
}

#[cfg(test)]
mod test {
    use kube_client::Client;
    use super::*;

    #[tokio::test]
    async fn test_lease() {
        let client = Client::try_default().await.unwrap();
        let lease = Lease::acquire_or_create(client, "staging", "cousteau", "test-case")
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_secs(5 * LEASE_DURATION_SECONDS as u64)).await;

        lease.join().await.unwrap();
    }

    #[tokio::test]
    async fn test_availability() {
        let client = Client::try_default().await.unwrap();
        let lease = Lease::acquire_or_create(client.clone(), "production", "cousteau", "test-case")
            .await
            .unwrap();

        let available = Lease::available(client, "production", "cousteau")
            .await
            .unwrap();
        panic!("available: {:#?}", available);
        drop(lease);
    }
}
