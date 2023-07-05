use super::future_hash_map::FutureHashMap;
use crate::scheduler::{ScheduleRequest, Scheduler};
use futures::{future, Future, FutureExt, Stream, StreamExt};
use pin_project::pin_project;
use std::{
    convert::Infallible,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<ReadyErr> {
    #[error("readiness gate failed to become ready")]
    Readiness(#[source] ReadyErr),
}

/// Pulls items from a [`Scheduler`], and runs an action for each item in parallel,
/// while making sure to not process [equal](`Eq`) items multiple times at once.
///
/// If an item is to be emitted from the [`Scheduler`] while an equal item is
/// already being processed then it will be held pending until the current item
/// is finished.
#[pin_project]
pub struct Runner<T, R, F, MkF, Ready = future::Ready<Result<(), Infallible>>> {
    #[pin]
    scheduler: Scheduler<T, R>,
    run_msg: MkF,
    slots: FutureHashMap<T, F>,
    #[pin]
    ready_to_execute_after: future::Fuse<Ready>,
    is_ready_to_execute: bool,
    stopped: bool,
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
            ready_to_execute_after: future::ready(Ok(())).fuse(),
            is_ready_to_execute: false,
            stopped: false,
        }
    }

    /// Wait for `ready_to_execute_after` to complete before starting to run any scheduled tasks.
    ///
    /// `scheduler` will still be polled in the meantime.
    pub fn delay_tasks_until<Ready, ReadyErr>(
        self,
        ready_to_execute_after: Ready,
    ) -> Runner<T, R, F, MkF, Ready>
    where
        Ready: Future<Output = Result<(), ReadyErr>>,
    {
        Runner {
            scheduler: self.scheduler,
            run_msg: self.run_msg,
            slots: self.slots,
            ready_to_execute_after: ready_to_execute_after.fuse(),
            is_ready_to_execute: false,
            stopped: false,
        }
    }
}

impl<T, R, F, MkF, Ready, ReadyErr> Stream for Runner<T, R, F, MkF, Ready>
where
    T: Eq + Hash + Clone + Unpin,
    R: Stream<Item = ScheduleRequest<T>>,
    F: Future + Unpin,
    MkF: FnMut(&T) -> F,
    Ready: Future<Output = Result<(), ReadyErr>>,
{
    type Item = Result<F::Output, Error<ReadyErr>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        if *this.stopped {
            return Poll::Ready(None);
        }
        let slots = this.slots;
        let scheduler = &mut this.scheduler;
        let has_active_slots = match slots.poll_next_unpin(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(Ok(result))),
            Poll::Ready(None) => false,
            Poll::Pending => true,
        };
        match this.ready_to_execute_after.poll(cx) {
            Poll::Ready(Ok(())) => *this.is_ready_to_execute = true,
            Poll::Ready(Err(err)) => {
                *this.stopped = true;
                return Poll::Ready(Some(Err(Error::Readiness(err))));
            }
            Poll::Pending => {}
        }
        loop {
            // Try to take take a new message that isn't already being processed
            // leave the already-processing ones in the queue, so that we can take them once
            // we're free again.
            let next_msg_poll = scheduler
                .as_mut()
                .hold_unless(|msg| *this.is_ready_to_execute && !slots.contains_key(msg))
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
    use super::{Error, Runner};
    use crate::{
        scheduler::{scheduler, ScheduleRequest},
        utils::delayed_init::{self, DelayedInit},
    };
    use futures::{
        channel::{mpsc, oneshot},
        future, poll, stream, SinkExt, StreamExt, TryStreamExt,
    };
    use std::{cell::RefCell, collections::HashSet, sync::Mutex, time::Duration};
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
            timeout(Duration::from_secs(1), result_rx)
                .await
                .unwrap()
                .unwrap()
                .transpose()
                .unwrap(),
            Some(8)
        );
    }

    #[tokio::test]
    async fn runner_should_wait_for_readiness() {
        let is_ready = Mutex::new(false);
        let (delayed_init, ready) = DelayedInit::<()>::new();
        let mut runner = Box::pin(
            Runner::new(
                scheduler(
                    stream::iter([ScheduleRequest {
                        message: 1u8,
                        run_at: Instant::now(),
                    }])
                    .chain(stream::pending()),
                ),
                |msg| {
                    assert!(*is_ready.lock().unwrap());
                    future::ready(*msg)
                },
            )
            .delay_tasks_until(ready.get()),
        );
        assert!(poll!(runner.next()).is_pending());
        *is_ready.lock().unwrap() = true;
        delayed_init.init(());
        assert_eq!(runner.next().await.transpose().unwrap(), Some(1));
    }

    #[tokio::test]
    async fn runner_should_dedupe_while_waiting_for_readiness() {
        let is_ready = Mutex::new(false);
        let (delayed_init, ready) = DelayedInit::<()>::new();
        let mut runner = Box::pin(
            Runner::new(
                scheduler(
                    stream::iter([
                        ScheduleRequest {
                            message: 1u8,
                            run_at: Instant::now(),
                        },
                        ScheduleRequest {
                            message: 2u8,
                            run_at: Instant::now(),
                        },
                        ScheduleRequest {
                            message: 1u8,
                            run_at: Instant::now(),
                        },
                    ])
                    .chain(stream::pending()),
                ),
                |msg| {
                    assert!(*is_ready.lock().unwrap());
                    future::ready(*msg)
                },
            )
            .delay_tasks_until(ready.get()),
        );
        assert!(poll!(runner.next()).is_pending());
        *is_ready.lock().unwrap() = true;
        delayed_init.init(());
        assert_eq!(
            runner.as_mut().take(2).try_collect::<HashSet<_>>().await.unwrap(),
            HashSet::from([1, 2])
        );
        assert!(poll!(runner.next()).is_pending());
    }

    #[tokio::test]
    async fn runner_should_report_readiness_errors() {
        let (delayed_init, ready) = DelayedInit::<()>::new();
        let mut runner = Box::pin(
            Runner::new(
                scheduler(
                    stream::iter([ScheduleRequest {
                        message: (),
                        run_at: Instant::now(),
                    }])
                    .chain(stream::pending()),
                ),
                |()| {
                    panic!("run_msg should never be invoked if readiness gate fails");
                    // It's "useless", but it helps to direct rustc to the correct types
                    #[allow(unreachable_code)]
                    future::ready(())
                },
            )
            .delay_tasks_until(ready.get()),
        );
        assert!(poll!(runner.next()).is_pending());
        drop(delayed_init);
        assert!(matches!(
            runner.try_collect::<Vec<_>>().await.unwrap_err(),
            Error::Readiness(delayed_init::InitDropped)
        ));
    }
}
