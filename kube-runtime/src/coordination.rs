//! Coordination utilities built around the `coordination.k8s.io/v1` API.
//!
//! This implementation uses only Kubernetes `coordination.k8s.io/v1/Lease` objects for
//! coordination. Every client running this `LeaderElector` task maintains a watch on the
//! lease object, and will receive updates as the lease holder heartbeats the lease.
//!
//! Applications using this API can use the spawn handle returned from spawning the
//! `LeaderElector` task to monitor lease state and to govern application behavior based on that
//! state. E.G.,
//!
//! ```rust,ignore
//! // Spawn a leader elector task, and get a handle to the state channel.
//! let handle = LeaderElector::spawn(/* ... */);
//! let state_chan = handle.state();
//!
//! // Before taking action as a leader, just check the channel to ensure
//! // the lease is currently held by this process.
//! if state_chan.borrow().is_leader() {
//!     // Only perform leader actions if in leader state.
//! }
//!
//! // Or, for a more sophisticated pattern, watch the state channel for changes,
//! // and use it to drive your application's state machine.
//! let state_stream = tokio_stream::wrappers::WatchStream::new(state_chan);
//! loop {
//!     tokio::select! {
//!         Some(state) = state_stream.next() => match state {
//!             LeaderState::Leader => (), // Leader tasks.
//!             _ => (), // Non-leader tasks.
//!         },
//!     }
//! }
//! ```
//!
//! ## Reference Implementation
//!
//! This implementation is based upon the upstream Kubernetes implementation in Go which can be
//! found here: <https://github.com/kubernetes/client-go/blob/2a6c116e406126324eee341e874612a5093bdbb0/tools/leaderelection/leaderelection.go>
//!
//! The following docs, adapted from the reference Go implementation, also apply here:
//!
//! > This implementation does not guarantee that only one client is acting as a leader (a.k.a. fencing).
//!
//! > A client only acts on timestamps captured locally to infer the state of the
//! > leader election. The client does not consider timestamps in the leader
//! > election record to be accurate because these timestamps may not have been
//! > produced by a local clock. The implemention does not depend on their
//! > accuracy and only uses their change to indicate that another client has
//! > renewed the leader lease. Thus the implementation is tolerant to arbitrary
//! > clock skew, but is not tolerant to arbitrary clock skew rate.
//! >
//! > However the level of tolerance to skew rate can be configured by setting
//! > `renew_deadline` and `lease_duration` appropriately. The tolerance expressed as a
//! > maximum tolerated ratio of time passed on the fastest node to time passed on
//! > the slowest node can be approximately achieved with a configuration that sets
//! > the same ratio of `lease_duration` to `renew_deadline`. For example if a user wanted
//! > to tolerate some nodes progressing forward in time twice as fast as other nodes,
//! > the user could set `lease_duration` to 60 seconds and `renew_deadline` to 30 seconds.
//! >
//! > While not required, some method of clock synchronization between nodes in the
//! > cluster is highly recommended. It's important to keep in mind when configuring
//! > this client that the tolerance to skew rate varies inversely to master
//! > availability.
//! >
//! > Larger clusters often have a more lenient SLA for API latency. This should be
//! > taken into account when configuring the client. The rate of leader transitions
//! > should be monitored and `retry_period` and `lease_duration` should be increased
//! > until the rate is stable and acceptably low. It's important to keep in mind
//! > when configuring this client that the tolerance to API latency varies inversely
//! > to master availability.

use std::time::{Duration, Instant};

use futures::prelude::*;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::MicroTime;
use rand::Rng;
use thiserror::Error;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::watcher::{watcher, Event, Result as WatcherResult};
use kube_client::api::{Api, ListParams, Patch, PatchParams};
use kube_client::{Client, Resource};

/// The jitter factor to use while attempting to acquire the lease.
const JITTER_FACTOR: f64 = 1.2;

/// Coordination error variants.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid leader election config: {0}")]
    ConfigError(String),
    #[error("timeout while updating api")]
    TimeoutError,
    #[error("client error from api call: {0}")]
    ClientError(kube_client::error::Error),
    #[error("error from the leader elector task: {0}")]
    TaskError(String),
}

/// Coordination result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Fully validated configuration for use by a `LeaderElector` instance.
///
/// Construct an instance via `ConfigBuilder::finish()`.
#[derive(Clone, Debug)]
pub struct Config(ConfigBuilder);

/// Configuration for leader election.
#[derive(Clone, Debug)]
pub struct ConfigBuilder {
    /// The name of the lease object.
    pub name: String,
    /// The namespace of the lease object.
    pub namespace: String,
    /// The identity to use when the lease is acquired.
    ///
    /// Typically this value will directly correspond to the name of the pod running this process.
    pub identity: String,
    /// The name to use for Server-Side Apply management group.
    ///
    /// Typically this value corresponds to the name of the group of controllers of which this
    /// leader elector is a part. **Note well** that this value should be the same across the
    /// entire management group and should be distinct from the `identity` parameter.
    pub manager: String,
    /// The duration that non-leader candidates will wait to force acquire leadership.
    /// This is measured against time of last observed ack.
    ///
    /// A client needs to wait a full `lease_duration` without observing a change to
    /// the record before it can attempt to take over. When all clients are
    /// shutdown and a new set of clients are started with different names against
    /// the same leader record, they must wait the full `lease_duration` before
    /// attempting to acquire the lease. Thus `lease_duration` should be as short as
    /// possible (within your tolerance for clock skew rate) to avoid possible
    /// long waits in such a scenario.
    ///
    /// Core clients default this value to 15 seconds.
    pub lease_duration: Duration,
    /// The duration that the current lease holder will retry refreshing the lease.
    ///
    /// Core clients default this value to 10 seconds.
    pub renew_deadline: Duration,
    /// The duration which leader elector clients should wait between tries of actions.
    ///
    /// Core clients default this value to 2 seconds.
    pub retry_period: Duration,
    /// API timeout to use for interacting with the K8s API.
    pub api_timeout: Duration,
}

impl ConfigBuilder {
    /// Finish building leader elector config by validating this config builder.
    ///
    /// # Errors
    /// Will return `Error::ConfigError` if this member's fields are invalid according to the
    /// following constraints:
    /// - `identity` must not be an empty string;
    /// - `manager` must not be an empty string;
    /// - `lease_duration` must be greater than `renew_deadline`;
    /// - `renew_deadline` must be greater than `(JITTER_FACTOR * retry_period.num_seconds())`;
    /// - `lease_duration` must be >= 1 second;
    /// - `renew_deadline` must be >= 1 second;
    /// - `retry_period` must be >= 1 second;
    /// - `apu_timeout` must be >= 1 second;
    pub fn finish(self) -> Result<Config> {
        if self.identity.is_empty() {
            return Err(Error::ConfigError("identity may not be empty".into()));
        }
        if self.manager.is_empty() {
            return Err(Error::ConfigError("manager may not be empty".into()));
        }
        if self.lease_duration <= self.renew_deadline {
            return Err(Error::ConfigError(
                "lease_duration must be greater than renew_deadline".into(),
            ));
        }
        if self.renew_deadline <= Duration::from_secs_f64(JITTER_FACTOR * self.retry_period.as_secs_f64()) {
            return Err(Error::ConfigError(format!(
                "renew_deadline must be greater than retry_period*{JITTER_FACTOR}"
            )));
        }
        if self.lease_duration.as_secs() < 1 {
            return Err(Error::ConfigError(
                "lease_duration must be at least 1 second".into(),
            ));
        }
        if self.renew_deadline.as_secs() < 1 {
            return Err(Error::ConfigError(
                "renew_deadline must be at least 1 second".into(),
            ));
        }
        if self.retry_period.as_secs() < 1 {
            return Err(Error::ConfigError(
                "retry_period must be at least 1 second".into(),
            ));
        }
        if self.api_timeout.as_secs() < 1 {
            return Err(Error::ConfigError("api_timeout must be at least 1 second".into()));
        }
        Ok(Config(self))
    }
}

/// A task which is responsible for acquiring and maintaining a `coordination.k8s.io/v1` `Lease`
/// to establish leadership.
pub struct LeaderElector {
    /// A K8s API wrapper around the client.
    api: Api<Lease>,
    /// Leader election config.
    config: ConfigBuilder,
    /// The internal state of this task.
    state: State,
    /// The state signal, which always reflects the current internal state of this task.
    state_tx: watch::Sender<LeaderState>,
    /// Shutdown channel.
    shutdown: oneshot::Receiver<()>,
    /// A bool indicating that there was an error encountered on the last attempt to acquire the lease.
    ///
    /// This is used as a simple retry / backoff indicator.
    had_error_on_last_try: bool,
}

impl LeaderElector {
    /// Create a new `LeaderElector` instance & spawn it onto the runtime for execution.
    #[must_use = "handle must be used for observing state changes and graceful shutdown"]
    pub fn spawn(config: Config, client: Client) -> LeaderElectorHandle {
        let (state_tx, state_rx) = watch::channel(LeaderState::Standby);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let this = LeaderElector {
            api: Api::namespaced(client, &config.0.namespace),
            config: config.0,
            state_tx,
            state: State::Standby,
            shutdown: shutdown_rx,
            had_error_on_last_try: false,
        };
        let handle = tokio::spawn(this.run());
        LeaderElectorHandle {
            shutdown: shutdown_tx,
            state: state_rx,
            handle,
        }
    }

    async fn run(mut self) {
        tracing::info!("leader elector task started");

        // Perform an initail pass at acquiring / renewing the lease.
        if let Err(err) = self.try_acquire_or_renew().await {
            tracing::error!(error = ?err, "error attempting to acquire/renew lease");
        }
        tracing::info!("finished initial call to try_acquire_or_renew");

        let lease_watcher = watcher(
            self.api.clone(),
            ListParams {
                field_selector: Some(format!("metadata.name={}", self.config.name)),
                ..Default::default()
            },
        );
        tokio::pin!(lease_watcher);

        loop {
            let delay_duration = self.get_next_acquire_renew_time();
            tracing::debug!("delaying for {}s", delay_duration.as_secs());
            let delay = tokio::time::sleep(delay_duration);
            tokio::select! {
                Some(lease_change_res) = lease_watcher.next() => self.k8s_handle_lease_event(lease_change_res).await,
                _ = delay => {
                    tracing::info!("delay elapsed, going to call try_acquire_or_renew");
                    if let Err(err) = self.try_acquire_or_renew().await {
                        tracing::error!(error = ?err, "error during call to try_acquire_or_renew");
                        self.had_error_on_last_try = true;
                        self.update_state(None);
                    }
                }
                _ = &mut self.shutdown => break,
            }
        }

        tracing::info!("leader elector task terminated");
    }

    /// Handle a change event from the lease watcher.
    #[tracing::instrument(level = "debug", skip_all)]
    async fn k8s_handle_lease_event(&mut self, res: WatcherResult<Event<Lease>>) {
        let event = match res {
            Ok(event) => event,
            Err(err) => {
                tracing::error!(error = ?err, "error from lease watcher stream");
                return;
            }
        };
        match event {
            Event::Applied(lease) => self.update_state(Some(lease)),
            Event::Restarted(leases) => {
                for lease in leases {
                    self.update_state(Some(lease));
                }
            }
            Event::Deleted(_) => {
                tracing::warn!(
                    "lease {}/{} unexpectedly deleted, will re-create",
                    self.config.namespace,
                    self.config.name
                );
                self.update_state(None);
            }
        };
    }

    /// Fetch the target lease from the API, and update observation info as needed.
    #[tracing::instrument(level = "debug", skip_all)]
    async fn k8s_fetch_lease_and_update(&mut self) -> Result<()> {
        // Attempt to fetch the target lease, updating our last observed info on the lease.
        let lease_opt = timeout(self.config.api_timeout, self.api.get_opt(&self.config.name))
            .await
            .map_err(|_err| Error::TimeoutError)?
            .map_err(Error::ClientError)?;

        // Lease info returned from the API, update observation info.
        if let Some(lease) = lease_opt {
            self.update_state(Some(lease));
            return Ok(());
        }
        Ok(())
    }

    /// Attempt to acquire or renew the target lease.
    #[tracing::instrument(level = "debug", skip_all, err)]
    #[allow(clippy::cast_possible_truncation)]
    async fn try_acquire_or_renew(&mut self) -> Result<()> {
        tracing::debug!("try_acquire_or_renew");
        // 1. Fetch the current state of the lease if following or standby.
        if matches!(&self.state, State::Following { .. } | State::Standby) {
            self.k8s_fetch_lease_and_update().await?;
        }

        // 2. Determine what type of update needs to be made to the lease.
        // If following, and the lease is not expired (according to our own local time records), then no-op.
        if matches!(&self.state, State::Following { .. }) && !self.is_lease_expired() {
            return Ok(());
        }
        // Else, we need to patch the lease. Build up changeset.
        let now = chrono::Utc::now();
        let mut lease = self.state.get_lease().cloned().unwrap_or_default();
        lease
            .meta_mut()
            .name
            .get_or_insert_with(|| self.config.name.clone());
        lease
            .meta_mut()
            .namespace
            .get_or_insert_with(|| self.config.namespace.clone());
        let spec = lease.spec.get_or_insert_with(Default::default);
        spec.lease_duration_seconds = Some(self.config.lease_duration.as_secs_f32() as i32);
        spec.renew_time = Some(MicroTime(chrono::Utc::now()));
        if matches!(&self.state, State::Following { .. } | State::Standby) {
            spec.holder_identity = Some(self.config.identity.clone());
            spec.acquire_time = Some(MicroTime(now));
            spec.lease_transitions = Some(spec.lease_transitions.map_or(0, |val| val + 1));
        }
        lease.metadata.managed_fields = None; // Can not pass this along for update.

        // 3. Now we need to create or patch the lease in K8s with the updated lease value here.
        let lease_res = if let State::Standby = &self.state {
            timeout(
                self.config.api_timeout,
                self.api.create(&Default::default(), &lease),
            )
            .await
        } else {
            let mut params = PatchParams::apply(&self.config.manager);
            params.force = true; // This will still be blocked by the server if we do not have the most up-to-date lease info.
            timeout(
                self.config.api_timeout,
                self.api.patch(&self.config.name, &params, &Patch::Apply(lease)),
            )
            .await
        };
        let lease = lease_res
            .map_err(|_err| Error::TimeoutError)?
            .map_err(Error::ClientError)?;
        self.update_state(Some(lease));

        Ok(())
    }

    /// Update task state based upon the given lease option.
    ///
    /// This will also handle updating this object's leadership state and will emit
    /// events as needed.
    #[tracing::instrument(level = "debug", skip_all)]
    fn update_state(&mut self, lease_opt: Option<Lease>) {
        // Unpack the given value, and if None then set state values to None.
        let updated_lease = if let Some(lease) = lease_opt {
            if Some(&lease) == self.state.get_lease() {
                return; // No change in lease, so simply return.
            }
            lease
        } else {
            self.state = State::Standby;
            let _res = self.state_tx.send_if_modified(|val| {
                if matches!(val, LeaderState::Standby) {
                    false
                } else {
                    *val = LeaderState::Standby;
                    true
                }
            });
            return;
        };

        // Process the given lease and update as needed.
        let holder = updated_lease
            .spec
            .as_ref()
            .and_then(|spec| spec.holder_identity.as_deref())
            .unwrap_or_default();
        let (is_lease_held, now) = (holder == self.config.identity, Instant::now());
        match &mut self.state {
            // In all cases where the lease is held by this identity, we simply update observed info.
            val @ (State::Leading { .. } | State::Following { .. } | State::Standby) if is_lease_held => {
                *val = State::Leading {
                    lease: updated_lease,
                    last_updated: now,
                };
            }
            // In any case where the holder ID is an empty string, we set to standby, as such a
            // config is invalid, and indicates that the lease is open for acquisition.
            val if holder.is_empty() => {
                *val = State::Standby;
            }
            // In all other cases, we are following, and we simply update observed info.
            val => {
                *val = State::Following {
                    leader: holder.into(),
                    lease: updated_lease,
                    last_updated: now,
                };
            }
        };
        self.state_tx.send_if_modified(|val| match &self.state {
            State::Leading { .. } if !matches!(val, LeaderState::Leading) => {
                *val = LeaderState::Leading;
                true
            }
            State::Following { .. } if !matches!(val, LeaderState::Following) => {
                *val = LeaderState::Following;
                true
            }
            State::Standby if !matches!(val, LeaderState::Standby) => {
                *val = LeaderState::Standby;
                true
            }
            _ => false,
        });
        tracing::debug!("lease updated");
    }

    /// Get the duration to delay before attempting the next lease update.
    fn get_next_acquire_renew_time(&mut self) -> Duration {
        // Unpack the last observed change and the configured delay times.
        let (last_updated, delay) = match &self.state {
            // If running as leader, then we renew a bit earlier then the lease duration.
            State::Leading { last_updated, .. } => (last_updated, self.config.renew_deadline),
            // If we are a follower, then we just wait to check based on configuration lease duration
            // and the last observed change. We also jitter to mitigate contention.
            State::Following { last_updated, .. } => {
                let rand_val: f64 = rand::thread_rng().gen_range(0.01..1.0);
                let jitter = rand_val * JITTER_FACTOR * self.config.lease_duration.as_secs_f64();
                let delay = self.config.lease_duration + Duration::from_secs_f64(jitter);
                (last_updated, delay)
            }
            // If an error recently took place, then we use the configured retry period plus a bit of jitter.
            State::Standby if self.had_error_on_last_try => {
                self.had_error_on_last_try = false;
                let rand_val: f64 = rand::thread_rng().gen_range(0.5..1.5);
                let jitter = rand_val * JITTER_FACTOR * self.config.retry_period.as_secs_f64();
                return Duration::from_secs_f64(jitter);
            }
            // If never observed, or recently delted, then we need to renew now.
            State::Standby => return Duration::from_secs(0),
        };

        // Establish how long we need to wait, and return the duration.
        let now = Instant::now();
        let deadline = *last_updated + delay;
        if deadline > now {
            // Deadline is in the future, so delay until deadline.
            deadline - now
        } else {
            // Else, time to renew now.
            Duration::from_secs(0)
        }
    }

    /// Check if the lease is expired.
    ///
    /// If the lease is unknown due to being in state `Standby`, this function will return `true`.
    fn is_lease_expired(&self) -> bool {
        match &self.state {
            State::Leading { last_updated, .. } | State::Following { last_updated, .. } => {
                let ttl = *last_updated + self.config.lease_duration;
                ttl <= Instant::now()
            }
            State::Standby => true,
        }
    }
}

/// The private state of the leader elector task.
#[derive(Clone, Debug, PartialEq)]
enum State {
    /// This client instance is the leader.
    Leading {
        /// The last observed lease state.
        lease: Lease,
        /// The last time this lease state updated as leader.
        last_updated: Instant,
    },
    /// A state indicating that a different client is currently the leader, identified by the
    /// encapsulated string.
    ///
    /// When a new leader is detected, this value will be updated with the leader's identity.
    Following {
        /// The ID of the current leader.
        leader: String,
        /// The last observed lease state.
        lease: Lease,
        /// The last time this lease state was observed.
        last_updated: Instant,
    },
    /// A state indicating that the lease state is unknown, does not exist, or that the
    /// corresponding leader elector task is starting or stopping.
    Standby,
}

impl State {
    /// Get a reference to the last known lease state.
    fn get_lease(&self) -> Option<&Lease> {
        match self {
            Self::Leading { lease, .. } | Self::Following { lease, .. } => Some(lease),
            Self::Standby => None,
        }
    }
}

/// Different states which a leader elector may be in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LeaderState {
    /// This client instance is the leader.
    Leading,
    /// A state indicating that a different client is currently the leader.
    Following,
    /// A state indicating that the lease state is unknown, does not exist, or that the
    /// corresponding leader elector task is starting or stopping.
    Standby,
}

impl LeaderState {
    /// Check if currently in `Leader` state.
    #[allow(clippy::must_use_candidate)]
    pub fn is_leader(&self) -> bool {
        matches!(self, Self::Leading)
    }
}

/// A handle to a leader elector task.
pub struct LeaderElectorHandle {
    /// Shutdown channel.
    shutdown: oneshot::Sender<()>,
    /// A watch signal over the observed leader state.
    state: watch::Receiver<LeaderState>,
    /// A join handle to the spawned leader elector task.
    handle: JoinHandle<()>,
}

impl LeaderElectorHandle {
    /// Get a handle to the state signal of this leader elector task.
    ///
    /// This signal receiver may be embedded in other parts of a program and used to govern actions
    /// taken by the app in accordance with leader election state.
    ///
    /// Note that this leader elector task itself, as well as any other controller group members using
    /// this same coordination implementation, will have an open watch on the lease object in the
    /// Kubernetes API, and thus will see lease updates as they take place. This helps to ensure a
    /// high degree of fencing, though it is not guaranteed.
    #[allow(clippy::must_use_candidate)]
    pub fn state(&self) -> watch::Receiver<LeaderState> {
        self.state.clone()
    }

    /// Shutdown this leader elector task and return its underlying join handle.
    #[allow(clippy::must_use_candidate)]
    pub fn shutdown(self) -> impl Future<Output = Result<()>> {
        let _res = self.shutdown.send(());
        self.handle.map_err(|res| Error::TaskError(res.to_string()))
    }
}
