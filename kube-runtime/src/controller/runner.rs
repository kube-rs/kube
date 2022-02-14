use super::future_hash_map::FutureHashMap;
use crate::scheduler::{ScheduleRequest, Scheduler};
use futures::{Future, Stream, StreamExt};
use pin_project::pin_project;
use std::{
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};

/// Pulls items from a [`Scheduler`], and runs an action for each item in parallel,
/// while making sure to not process [equal](`Eq`) items multiple times at once.
///
/// If an item is to be emitted from the [`Scheduler`] while an equal item is
/// already being processed then it will be held pending until the current item
/// is finished.
#[pin_project]
pub struct Runner<T, R, F, MkF> {
    #[pin]
    scheduler: Scheduler<T, R>,
    run_msg: MkF,
    slots: FutureHashMap<T, F>,
}

impl<T, R, F, MkF> Runner<T, R, F, MkF>
where
    F: Future + Unpin,
    MkF: FnMut(&T) -> F,
{
    pub fn new(scheduler: Scheduler<T, R>, run_msg: MkF) -> Self {
        Self {
            scheduler,
            run_msg,
            slots: FutureHashMap::default(),
        }
    }
}

impl<T, R, F, MkF> Stream for Runner<T, R, F, MkF>
where
    T: Eq + Hash + Clone + Unpin,
    R: Stream<Item = ScheduleRequest<T>>,
    F: Future + Unpin,
    MkF: FnMut(&T) -> F,
{
    type Item = F::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let slots = this.slots;
        let scheduler = &mut this.scheduler;
        let has_active_slots = match slots.poll_next_unpin(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
            Poll::Ready(None) => false,
            Poll::Pending => true,
        };
        loop {
            // Try to take take a new message that isn't already being processed
            // leave the already-processing ones in the queue, so that we can take them once
            // we're free again.
            let next_msg_poll = scheduler
                .as_mut()
                .hold_unless(|msg| !slots.contains_key(msg))
                .poll_next_unpin(cx);
            match next_msg_poll {
                Poll::Ready(Some(msg)) => {
                    let msg_fut = (this.run_msg)(&msg);
                    assert!(
                        slots.insert(msg, msg_fut).is_none(),
                        "Runner tried to replace a running future.. please report this as a kube-rs bug!"
                    );
                    cx.waker().wake_by_ref();
                }
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
    use futures::{
        channel::{mpsc, oneshot},
        future, poll, SinkExt, StreamExt,
    };
    use std::{cell::RefCell, time::Duration};
    use tokio::{
        runtime::Handle,
        task::yield_now,
        time::{pause, sleep, timeout, Instant},
    };

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
                Box::pin(async move {
                    sleep(Duration::from_secs(1)).await;
                    drop(mutex_ref);
                })
            })
            .for_each(|_| async {}),
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
        future::join(
            async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                drop(sched_tx);
            },
            runner,
        )
        .await;
        // Validate that we actually ran both requests
        assert_eq!(count, 2);
    }

    // Test MUST be single-threaded to be consistent, since it concerns a relatively messy
    // interplay between multiple tasks
    #[tokio::test(flavor = "current_thread")]
    async fn runner_should_wake_when_scheduling_messages() {
        // pause();
        let (mut sched_tx, sched_rx) = mpsc::unbounded();
        let (result_tx, result_rx) = oneshot::channel();
        let mut runner = Runner::new(scheduler(sched_rx), |msg: &u8| futures::future::ready(*msg));
        // Start a background task that starts listening /before/ we enqueue the message
        // We can't just use Stream::poll_next(), since that bypasses the waker system
        Handle::current().spawn(async move { result_tx.send(runner.next().await).unwrap() });
        // Ensure that the background task actually gets to initiate properly, and starts polling the runner
        yield_now().await;
        sched_tx
            .send(ScheduleRequest {
                message: 8,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        // Eventually the background task should finish up and report the message received,
        // a timeout here *should* mean that the background task isn't getting awoken properly
        // when the new message is ready.
        assert_eq!(
            timeout(Duration::from_secs(1), result_rx).await.unwrap().unwrap(),
            Some(8)
        );
    }
}
