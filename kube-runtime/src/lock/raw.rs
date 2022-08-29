//! Low-level lock implementation

use k8s_openapi::{
    api::coordination::v1::Lease,
    apimachinery::pkg::apis::meta::v1::MicroTime,
    chrono::{DateTime, Duration, Utc},
};
use kube_client::{core::ObjectMeta, Api};

pub struct LockSettings {
    pub lease_name: String,
    /// Identity is a string which uniquely determines actor among all other
    /// actors using same lease in same namespace.
    /// If two or more actors use the same identity, "at most one" property
    /// will be completely violated (if one actor will acquire lock, other will also believe
    /// it acquired it).
    pub identity: String,
    pub expiration_timeout_secs: i32,
}

#[derive(Clone)]
struct State(Option<Lease>);

/// Pure type that only manipulates lease state.
/// It always assumes that it is given latest and up-to-date
/// lease state. If this assumption turns wrong, compare-and-set simply fails.
struct StateEditor(LockSettings);

impl StateEditor {
    fn is_eligible_for_acquire(&self, state: &State, now: DateTime<Utc>) -> bool {
        let lease = match state.0.as_ref() {
            Some(l) => l,
            // lease not exists -> no holder -> can acquire
            None => return true,
        };
        let spec = match lease.spec.as_ref() {
            Some(s) => s,
            // empty spec -> no holder -> can acquire
            None => return true,
        };
        let holder = match spec.holder_identity.as_ref() {
            Some(h) => h,
            // no holder -> can acquire
            None => return true,
        };
        if *holder == self.0.identity {
            // held by us -> can acquire (aka renew)
            return true;
        }
        let last_renewed_at = match spec.renew_time.clone() {
            Some(t) => t,
            // no renew_time -> no holder -> can acquire
            None => return true,
        };
        let expires_at =
            last_renewed_at.0 + Duration::seconds(spec.lease_duration_seconds.unwrap_or(0).into());
        expires_at < now
    }

    fn acquire(&self, state: State, now: DateTime<Utc>) -> Result<Lease, State> {
        if !self.is_eligible_for_acquire(&state, now) {
            tracing::debug!("lock is acquired by other actor and did not expire yet");
            return Err(state);
        }
        let mut lease = state.0.unwrap_or_else(|| Lease {
            metadata: ObjectMeta {
                name: Some(self.0.lease_name.clone()),
                ..Default::default()
            },
            spec: None,
        });
        let spec = lease.spec.get_or_insert_with(Default::default);
        spec.renew_time = Some(MicroTime(now));
        let prev_holder = spec.holder_identity.replace(self.0.identity.clone());
        if prev_holder.as_ref() != Some(&self.0.identity) {
            spec.acquire_time = Some(MicroTime(now));
            let transitions_cnt = spec.lease_transitions.map_or(0, |cnt| cnt.saturating_add(1));
            spec.lease_transitions = Some(transitions_cnt);
        }
        spec.lease_duration_seconds = Some(self.0.expiration_timeout_secs);
        Ok(lease)
    }

    fn release(&self, mut state: State) -> Result<Lease, State> {
        let lease = match &mut state.0 {
            Some(l) => l,
            // we can't own lock if lease not exists
            None => return Err(state),
        };
        let spec = match lease.spec.as_mut() {
            Some(s) => s,
            // we can't own lock if spec is None
            None => return Err(state),
        };
        if spec.holder_identity.as_ref() != Some(&self.0.identity) {
            // we do not own lock
            return Err(state);
        }
        spec.holder_identity = None;
        spec.acquire_time = None;
        spec.renew_time = None;
        spec.lease_duration_seconds = None;
        Ok(state.0.unwrap())
    }
}

// This is the only type that directly communicates with k8s.
// it also maintains state cache.
struct StateHolder {
    leases: Api<Lease>,
    lease_name: String,
    // None: we **definitely** don't know current state (e.g. replace failed with Conflict).
    // Some(State(None)): we believe lease does not exist.
    // Some(State(Some(l))): we believe lease is l.
    state: Option<State>,
    // Unlike `state`, this field is never set to None once it becomes Some.
    last_observed_lease: Option<Lease>,
}

enum CasResult {
    Cancelled,
    Replaced,
    Conflict,
}

impl StateHolder {
    async fn get_state(&mut self) -> Result<State, kube_client::Error> {
        let val = match self.leases.get(&self.lease_name).await {
            Ok(lease) => Some(lease),
            Err(err) => {
                if is_api_error(&err, "NotFound") {
                    None
                } else {
                    return Err(err);
                }
            }
        };
        self.last_observed_lease = val.clone();
        Ok(State(val))
    }

    async fn compare_and_set_inner(
        &self,
        f: &dyn Fn(State) -> Result<Lease, State>,
        current_state: State,
    ) -> Result<(CasResult, Option<State>), kube_client::Error> {
        let lease_exists = current_state.0.is_some();
        let new_lease = match f(current_state) {
            Ok(l) => l,
            Err(state) => {
                return Ok((CasResult::Cancelled, Some(state)));
            }
        };

        tracing::debug!("Running compare-and-set");
        let result = if lease_exists {
            self.leases
                .replace(&self.lease_name, &Default::default(), &new_lease)
                .await
        } else {
            self.leases.create(&Default::default(), &new_lease).await
        };
        match result {
            Ok(lease) => {
                // success
                Ok((CasResult::Replaced, Some(State(Some(lease)))))
            }
            Err(err) => {
                if is_api_error(&err, "Conflict") || is_api_error(&err, "NotFound") {
                    // our state was not up-to-date
                    Ok((CasResult::Conflict, None))
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn compare_and_set<F>(&mut self, func: F) -> Result<CasResult, kube_client::Error>
    where
        F: Fn(State) -> Result<Lease, State>,
    {
        let state = match self.state.take() {
            Some(s) => s,
            None => self.get_state().await?,
        };
        let (cas_result, state) = self.compare_and_set_inner(&func, state).await?;
        if let Some(State(Some(lease))) = &state {
            self.last_observed_lease = Some(lease.clone());
        }
        self.state = state;
        Ok(cas_result)
    }

    fn last_observed_lease(&self) -> Option<&Lease> {
        self.last_observed_lease.as_ref()
    }
}

/// Raw lock.
///
/// You should not use this type directly, because it has error-prone and verbose API.
/// Use other primitives instead: (TODO).
/// # Limitations
/// - `RawLock` does not enforce that work only happens in critical section.
/// - `RawLock` does not notify about its expiration.
/// - `RawLock` must be repeatedly explicitly `acquire()`-d for renewal to work.
#[allow(clippy::module_name_repetitions)] // OK since module is private
pub struct RawLock {
    editor: StateEditor,
    state: StateHolder,
}

fn is_api_error(err: &kube_client::Error, reason: &str) -> bool {
    let err = match err {
        kube_client::Error::Api(e) => e,
        _ => return false,
    };
    err.reason == reason
}

impl RawLock {
    /// Creates new `RawLock` (i.e. one actor in distributed locking problem).
    #[must_use]
    pub fn new(leases: Api<Lease>, settings: LockSettings) -> Self {
        let lease_name = settings.lease_name.clone();
        RawLock {
            editor: StateEditor(settings),
            state: StateHolder {
                leases,
                lease_name,
                state: None,
                last_observed_lease: None,
            },
        }
    }

    /// Returns last observed term of the lock or -1 otherwise. During normal operation,
    /// this term monotonically increases each time lock changes owner or is re-acquired after an.
    /// explicit release.
    /// No two actors may believe they acquired the lock in one term.
    #[must_use]
    pub fn term(&self) -> i32 {
        self.state
            .last_observed_lease()
            .and_then(|lease| lease.spec.as_ref())
            .and_then(|spec| spec.lease_transitions)
            .unwrap_or(-1)
    }

    /// Returns last known owner of the lock.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.state
            .last_observed_lease()
            .and_then(|lease| lease.spec.as_ref())
            .and_then(|s| s.holder_identity.as_deref())
    }

    /// Returns moment of time when last observed lock will expire.
    #[must_use]
    pub fn locked_until(&self) -> DateTime<Utc> {
        self.state
            .last_observed_lease()
            .and_then(|lease| lease.spec.as_ref())
            .and_then(|s| Option::zip(s.renew_time.clone(), s.lease_duration_seconds))
            .map_or(DateTime::<Utc>::MIN_UTC, |(last_renewed_at, duration)| {
                last_renewed_at.0 + Duration::seconds(duration.into())
            })
    }

    /// If lock is released or stale, acquires it and returns true.
    /// If lock is acquired by this actor, renews it and returns true.
    /// Otherwise returns false.
    /// This function may return false spuriously if not only `RawLock` actors
    /// modify the lease.
    /// # Errors
    /// This function fails if apiserver returns unexpected error.
    pub async fn try_acquire(&mut self, now: DateTime<Utc>) -> Result<bool, kube_client::Error> {
        tracing::debug!("trying to acquire or renew lock");
        let func = |state: State| self.editor.acquire(state, now);
        let res = self.state.compare_and_set(func).await?;
        match res {
            CasResult::Replaced => {
                tracing::debug!("lock was successfully acquired or renewed");
                Ok(true)
            }
            CasResult::Conflict => {
                tracing::debug!("Acquire operation failed due to CAS conflict");
                Ok(false)
            }
            CasResult::Cancelled => Ok(false),
        }
    }

    /// If lock is acquired by this actor, releases it.
    /// Returns true if lock is not owned anymore, false otherwise.
    /// # Errors
    /// This function fails if apiserver returns unexpected error.
    pub async fn try_release(&mut self) -> Result<bool, kube_client::Error> {
        tracing::debug!("trying to release lock");
        let func = |state: State| self.editor.release(state);
        let res = self.state.compare_and_set(func).await?;
        match res {
            CasResult::Replaced => {
                tracing::debug!("Lock was successfully released");
                Ok(true)
            }
            CasResult::Conflict => {
                tracing::debug!("Release operation failed due to CAS conflict");
                Ok(false)
            }
            CasResult::Cancelled => {
                tracing::debug!("Lock was not owned");
                Ok(true)
            }
        }
    }
}
