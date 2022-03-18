#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

mod sleep;

use std::{convert::Infallible, pin::Pin};

use futures::{
    future::{self, Either},
    pin_mut, Future, Stream, StreamExt, TryStream,
};
use k8s_openapi::{
    api::coordination::v1::{Lease, LeaseSpec},
    apimachinery::pkg::apis::meta::v1::MicroTime,
    chrono::{DateTime, Duration, Utc, MIN_DATETIME},
};
use kube_client::Api;
use pin_project::pin_project;

use crate::watcher::{self, watch_object};

pub struct Elector {
    api: Api<Lease>,
    name: String,
    identity: String,
    lease_duration_secs: i32,
}

impl Elector {
    #[must_use]
    pub fn new(api: Api<Lease>, lease: &str, instance: &str, lease_duration_secs: i32) -> Self {
        Self {
            api,
            name: lease.to_string(),
            identity: instance.to_string(),
            lease_duration_secs,
        }
    }

    #[tracing::instrument(skip(self, fut))]
    pub async fn run<F: Future>(&self, fut: F) -> Result<F::Output, RunError> {
        dbg!(self.acquire().await.map_err(RunError::Acquire))?;
        let renewer = self.keep_renewed();
        pin_mut!(renewer, fut);
        let output = match future::select(renewer, fut).await {
            Either::Left((err, _)) => return Err(RunError::Renew(err)),
            Either::Right((output, _)) => output,
        };
        self.release().await.map_err(RunError::Release)?;
        Ok(output)
    }

    #[tracing::instrument(skip(self))]
    async fn keep_renewed(&self) -> RenewError {
        let watcher = watch_object(self.api.clone(), &self.name);
        let active_renewal = future::Either::Left(future::pending());
        pin_mut!(watcher, active_renewal);
        loop {
            match future::select(watcher.next(), active_renewal.as_mut()).await {
                // Lease watcher
                Either::Left((None, _)) => panic!("watcher should never terminate"),
                Either::Left((Some(Err(err)), _)) => return RenewError::Watch(err),
                Either::Left((Some(Ok(lease)), _)) => {
                    let now = Utc::now();
                    let lease_state = self.state(&lease.and_then(|l| l.spec).unwrap_or_default());
                    if let LeaseState::HeldBySelf { renew_at, expires_at } = lease_state {
                        active_renewal.set(future::Either::Right(sleep::with_deadline(
                            expires_at,
                            async move {
                                tracing::info!(%renew_at, "scheduling next renewal...");
                                sleep::until(renew_at).await;
                                tracing::debug!("renewing");
                                self.try_acquire(now).await?;
                                tracing::debug!("renew finished");
                                // watcher should emit the new lease, scheduling a new renewal
                                Ok(future::pending::<Infallible>().await)
                            },
                        )))
                    } else {
                        return RenewError::Lost;
                    }
                }

                // Renewer
                Either::Right((Err(deadline_expired), _)) => return RenewError::Timeout(deadline_expired),
                Either::Right((Ok(Err(renew_err)), _)) => return RenewError::Acquire(renew_err),
                Either::Right((Ok(Ok(x)), _)) => match x {},
            }
        }
    }

    #[tracing::instrument(skip(self, watcher))]
    async fn acquire(
        &self,
        watcher: Pin<&mut StreamCache<impl Stream<Item = watcher::Result<Option<Lease>>>>>,
    ) -> Result<(), AcquireError> {
        // let watcher = watch_object(self.api.clone(), &self.name);
        let mut watcher = watcher.recv();
        let active_acquisition = future::Either::Left(future::pending());
        pin_mut!(watcher, active_acquisition);
        loop {
            match future::select(watcher.next(), active_acquisition.as_mut()).await {
                Either::Left((watcher_evt, _)) => match watcher_evt {
                    None => panic!("watcher should never terminate"),
                    Some(Err(err)) => return Err(AcquireError::Watch(err)),
                    Some(Ok(lease)) => {
                        let lease_state = self.state(&lease.and_then(|l| l.spec).unwrap_or_default());

                        if let LeaseState::HeldBySelf { expires_at, .. } = lease_state {
                            if expires_at > Utc::now() {
                                return Ok(());
                            }
                        }

                        active_acquisition.set(future::Either::Right(async move {
                            if let LeaseState::HeldByOther { holder, expires_at } = lease_state {
                                tracing::info!(?holder, %expires_at, "scheduling next acquisition attempt...");
                                sleep::until(expires_at).await;
                            }
                            tracing::debug!("acquiring");
                            self.try_acquire(Utc::now()).await?;
                            tracing::debug!("acquisition finished");
                            Ok(())
                        }))
                    }
                },
                Either::Right((attempt_result, _)) => match dbg!(attempt_result) {
                    Err(TryAcquireError::Acquire(err)) => return Err(err),
                    Ok(()) | Err(TryAcquireError::Conflict { .. }) => {
                        // watcher should emit the new lease, triggering a successful return or re-check
                        active_acquisition.set(future::Either::Left(future::pending()));
                    }
                },
            }
        }
    }

    #[tracing::instrument(skip(self, now))]
    async fn try_acquire(&self, now: DateTime<Utc>) -> Result<(), TryAcquireError> {
        let mut entry = self
            .api
            .entry(&self.name)
            .await
            .map_err(AcquireError::Get)
            .map_err(TryAcquireError::Acquire)?
            .or_insert(Lease::default);
        let lease = entry.get_mut().spec.get_or_insert_with(LeaseSpec::default);
        let lease_state = self.state(lease);

        if let LeaseState::HeldByOther {
            ref holder,
            expires_at,
        } = lease_state
        {
            if expires_at > now {
                return Err(TryAcquireError::Conflict {
                    holder: holder.clone(),
                    expires_at,
                });
            }
        }

        if !matches!(lease_state, LeaseState::HeldBySelf { .. }) {
            lease.holder_identity = Some(self.identity.clone());
            lease.acquire_time = Some(MicroTime(now));
            *lease.lease_transitions.get_or_insert(0) += 1;
        }
        lease.renew_time = Some(MicroTime(now));
        lease.lease_duration_seconds = Some(self.lease_duration_secs);

        entry
            .commit()
            .await
            .map_err(AcquireError::Commit)
            .map_err(TryAcquireError::Acquire)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn release(&self) -> Result<(), ReleaseError> {
        let mut entry = self
            .api
            .entry(&self.name)
            .await
            .map_err(ReleaseError::Get)?
            .or_insert(Lease::default);
        let lease = entry.get_mut().spec.get_or_insert_with(LeaseSpec::default);
        match self.state(lease) {
            LeaseState::Unheld => Ok(()),
            LeaseState::HeldByOther { holder, .. } => Err(ReleaseError::AlreadyStolen { holder }),
            LeaseState::HeldBySelf { .. } => {
                lease.holder_identity = None;
                lease.acquire_time = None;
                lease.renew_time = None;
                lease.lease_duration_seconds = None;
                *lease.lease_transitions.get_or_insert(0) += 1;
                entry.commit().await.map_err(ReleaseError::Commit)?;
                Ok(())
            }
        }
    }

    fn state(&self, lease: &LeaseSpec) -> LeaseState {
        let lease_duration = Duration::seconds(lease.lease_duration_seconds.unwrap_or(0).into());
        let last_renewal = lease.renew_time.as_ref().map_or(MIN_DATETIME, |dt| dt.0);

        match &lease.holder_identity {
            None => LeaseState::Unheld,
            Some(holder) if holder == &self.identity => LeaseState::HeldBySelf {
                expires_at: last_renewal + lease_duration,
                renew_at: last_renewal + lease_duration * 2,
            },
            Some(holder) => LeaseState::HeldByOther {
                holder: holder.clone(),
                expires_at: last_renewal + lease_duration,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LeaseState {
    Unheld,
    HeldByOther {
        holder: String,
        expires_at: DateTime<Utc>,
    },
    HeldBySelf {
        renew_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Debug)]
pub enum AcquireError {
    Watch(watcher::Error),
    Get(kube_client::Error),
    Commit(kube_client::api::entry::CommitError),
}

#[derive(Debug)]
pub enum TryAcquireError {
    Acquire(AcquireError),
    Conflict {
        holder: String,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Debug)]
pub enum ReleaseError {
    Get(kube_client::Error),
    Commit(kube_client::api::entry::CommitError),
    AlreadyStolen { holder: String },
}

#[derive(Debug, thiserror::Error)]
pub enum RenewError {
    #[error("failed to watch lease")]
    Watch(#[source] watcher::Error),
    #[error("failed to acquire lease")]
    Acquire(TryAcquireError),
    #[error("lease expired before it could be renewed")]
    Timeout(#[source] sleep::DeadlineExpired),
    #[error("lease was lost")]
    Lost,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("failed to acquire lease")]
    Acquire(AcquireError),
    #[error("failed to renew lease")]
    Renew(RenewError),
    #[error("failed to release lease")]
    Release(ReleaseError),
}

#[pin_project]
struct StreamCache<S: Stream> {
    #[pin]
    stream: S,
    last_item: Option<S::Item>,
}

impl<S: Stream> StreamCache<S> {
    fn recv(self: Pin<&mut Self>) -> StreamCacheRecv<S> {
        StreamCacheRecv {
            cache: self,
            emitted_first_item: false,
        }
    }
}

struct StreamCacheRecv<'a, S: Stream> {
    cache: Pin<&'a mut StreamCache<S>>,
    emitted_first_item: bool,
}

impl<'a, S: Stream> StreamCacheRecv<'a, S> {
    async fn next(&mut self) -> Option<&mut S::Item> {
        let mut cache = self.cache.as_mut().project();

        if self.emitted_first_item || cache.last_item.is_none() {
            *cache.last_item = cache.stream.next().await;
        }
        self.emitted_first_item = true;
        cache.last_item.as_mut()
    }
}
