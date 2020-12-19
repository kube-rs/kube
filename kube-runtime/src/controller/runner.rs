use super::stream_hash_map::StreamHashMap;
use crate::scheduler::{self, ScheduleRequest, Scheduler};
use futures::{Future, Stream, StreamExt};
use pin_project::pin_project;
use std::{
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};

/// Pulls messages from a [`Scheduler`], and runs an action for each message in parallel,
/// while making sure to not run the same message multiple times at once.
#[pin_project]
pub struct Runner<T, R, F, MkF> {
    #[pin]
    scheduler: Scheduler<T, R>,
    run_msg: MkF,
    slots: StreamHashMap<T, futures::stream::Once<Pin<Box<F>>>>,
}

impl<T, R, F, MkF> Runner<T, R, F, MkF>
where
    F: Future,
    MkF: FnMut(&T) -> F,
{
    pub fn new(scheduler: Scheduler<T, R>, run_msg: MkF) -> Self {
        Self {
            scheduler,
            run_msg,
            slots: StreamHashMap::default(),
        }
    }
}

impl<T, R, F, MkF> Stream for Runner<T, R, F, MkF>
where
    T: Eq + Hash + Clone + Unpin,
    R: Stream<Item = ScheduleRequest<T>>,
    F: Future,
    MkF: FnMut(&T) -> F,
{
    type Item = scheduler::Result<F::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let slots = this.slots;
        let scheduler = &mut this.scheduler;
        let has_active_slots = match slots.poll_next_unpin(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(Ok(result))),
            Poll::Ready(None) => false,
            Poll::Pending => true,
        };
        loop {
            let next_msg_poll = scheduler
                .as_mut()
                .hold_unless(|msg| !slots.contains_key(msg))
                .poll_next_unpin(cx);
            match next_msg_poll {
                Poll::Ready(Some(Ok(msg))) => {
                    let msg_fut = Box::pin((this.run_msg)(&msg));
                    assert!(
                        !slots.insert_future(msg, msg_fut),
                        "Runner tried to replace a running future.. please report this as a kube-rs bug!"
                    );
                }
                Poll::Ready(Some(Err(err))) => break Poll::Ready(Some(Err(err))),
                Poll::Ready(None) => {
                    break if has_active_slots {
                        // We're done listening for new messages, but still have some that
                        // haven't finished quite yet
                        Poll::Pending
                    } else {
                        Poll::Ready(None)
                    };
                }
                Poll::Pending => break Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Runner;
    use crate::scheduler::{scheduler, ScheduleRequest};
    use futures::{channel::mpsc, poll, SinkExt, TryStreamExt};
    use std::{cell::RefCell, time::Duration};
    use tokio::time::{delay_for, pause, Instant};

    #[tokio::test]
    async fn runner_should_never_run_two_instances_at_once() {
        pause();
        let rc = RefCell::new(());
        let mut count = 0;
        let (mut sched_tx, sched_rx) = mpsc::unbounded();
        let mut runner = Box::pin(
            Runner::new(scheduler(sched_rx), |_| {
                count += 1;
                // Panic if this ref is already held, to simulate some unsafe action..
                let mutex_ref = rc.borrow_mut();
                async move {
                    delay_for(Duration::from_secs(1)).await;
                    drop(mutex_ref);
                }
            })
            .try_for_each(|_| async { Ok(()) }),
        );
        sched_tx
            .send(ScheduleRequest {
                message: (),
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        assert!(poll!(runner.as_mut()).is_pending());
        sched_tx
            .send(ScheduleRequest {
                message: (),
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        drop(sched_tx);
        runner.await.unwrap();
        // Validate that we actually ran both requests
        assert_eq!(count, 2);
    }
}
