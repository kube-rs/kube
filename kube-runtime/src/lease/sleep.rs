use futures::{future, pin_mut, Future};
use k8s_openapi::chrono::{DateTime, Utc};

pub async fn until(dt: DateTime<Utc>) {
    match (dt - Utc::now()).to_std() {
        Ok(duration) => {
            tokio::time::sleep(duration).await;
        }
        Err(_) => {
            tracing::trace!(%dt, "tried to wait until time that has already passed");
        }
    }
}

pub async fn with_deadline<F: Future>(deadline: DateTime<Utc>, f: F) -> Result<F::Output, DeadlineExpired> {
    let deadline_sleep = until(deadline);
    pin_mut!(deadline_sleep, f);
    match future::select(deadline_sleep, f).await {
        future::Either::Left(((), _)) => Err(DeadlineExpired { deadline }),
        future::Either::Right((out, _)) => Ok(out),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("deadline expired: {deadline}")]
pub struct DeadlineExpired {
    deadline: DateTime<Utc>,
}
