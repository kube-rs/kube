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
    max_concurrent_executions: u16,
}

impl<T, R, F, MkF> Runner<T, R, F, MkF>
where
    F: Future + Unpin,
    MkF: FnMut(&T) -> F,
{
    /// Creates a new [`Runner`]. [`max_concurrent_executions`] can be used to
    /// limit the number of items are run concurrently. It can be set to 0 to
    /// allow for unbounded concurrency.
    pub fn new(scheduler: Scheduler<T, R>, max_concurrent_executions: u16, run_msg: MkF) -> Self {
        Self {
            scheduler,
            run_msg,
            slots: FutureHashMap::default(),
            ready_to_execute_after: future::ready(Ok(())).fuse(),
            is_ready_to_execute: false,
            stopped: false,
            max_concurrent_executions,
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
            max_concurrent_executions: self.max_concurrent_executions,
        }
    }
}

#[allow(clippy::match_wildcard_for_single_variants)]
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
            // If we are at our limit or not ready to start executing, then there's
            // no point in trying to get something from the scheduler, so just put
            // all expired messages emitted from the queue into pending.
            if (*this.max_concurrent_executions > 0
                && slots.len() >= *this.max_concurrent_executions as usize)
                || !*this.is_ready_to_execute
            {
                match scheduler.as_mut().hold().poll_next_unpin(cx) {
                    Poll::Pending | Poll::Ready(None) => break Poll::Pending,
                    // The above future never returns Poll::Ready(Some(_)).
                    _ => unreachable!(),
                };
            };

            // Try to take a new message that isn't already being processed
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
    use super::{Error, Runner};
    use crate::{
        scheduler::{scheduler, ScheduleRequest},
        utils::delayed_init::{self, DelayedInit},
    };
    use futures::{
        channel::{mpsc, oneshot},
        future::{self},
        poll, stream, Future, SinkExt, StreamExt, TryStreamExt,
    };
    use std::{
        cell::RefCell,
        collections::HashMap,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::Duration,
    };
    use tokio::{
        runtime::Handle,
        task::yield_now,
        time::{advance, pause, sleep, timeout, Instant},
    };

    #[tokio::test]
    async fn runner_should_never_run_two_instances_at_once() {
        pause();
        let rc = RefCell::new(());
        let mut count = 0;
        let (mut sched_tx, sched_rx) = mpsc::unbounded();
        let mut runner = Box::pin(
            // The debounce period needs to zero because a debounce period > 0
            // will lead to the second request to be discarded.
            Runner::new(scheduler(sched_rx), 0, |_| {
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
        let mut runner = Runner::new(scheduler(sched_rx), 0, |msg: &u8| futures::future::ready(*msg));
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
                0,
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
                            message: 'a',
                            run_at: Instant::now(),
                        },
                        ScheduleRequest {
                            message: 'b',
                            run_at: Instant::now(),
                        },
                        ScheduleRequest {
                            message: 'a',
                            run_at: Instant::now(),
                        },
                    ])
                    .chain(stream::pending()),
                ),
                0,
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
        let mut message_counts = HashMap::new();
        assert!(timeout(
            Duration::from_secs(1),
            runner.try_for_each(|msg| {
                *message_counts.entry(msg).or_default() += 1;
                async { Ok(()) }
            })
        )
        .await
        .is_err());
        assert_eq!(message_counts, HashMap::from([('a', 1), ('b', 1)]));
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
                0,
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

    // A Future that is Ready after the specified duration from its initialization.
    struct DurationalFuture {
        start: Instant,
        ready_after: Duration,
    }

    impl DurationalFuture {
        fn new(expires_in: Duration) -> Self {
            let start = Instant::now();
            DurationalFuture {
                start,
                ready_after: expires_in,
            }
        }
    }

    impl Future for DurationalFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let now = Instant::now();
            if now.duration_since(self.start) > self.ready_after {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    #[tokio::test]
    async fn runner_should_respect_max_concurrent_executions() {
        pause();

        let count = Arc::new(Mutex::new(0));
        let (mut sched_tx, sched_rx) = mpsc::unbounded();
        let mut runner = Box::pin(
            Runner::new(scheduler(sched_rx), 2, |_| {
                let mut num = count.lock().unwrap();
                *num += 1;
                DurationalFuture::new(Duration::from_secs(2))
            })
            .for_each(|_| async {}),
        );

        sched_tx
            .send(ScheduleRequest {
                message: 1,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        assert!(poll!(runner.as_mut()).is_pending());
        sched_tx
            .send(ScheduleRequest {
                message: 2,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        assert!(poll!(runner.as_mut()).is_pending());
        sched_tx
            .send(ScheduleRequest {
                message: 3,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        assert!(poll!(runner.as_mut()).is_pending());
        // Assert that we only ran two out of the three requests
        assert_eq!(*count.lock().unwrap(), 2);

        advance(Duration::from_secs(3)).await;
        assert!(poll!(runner.as_mut()).is_pending());
        // Assert that we run the third request when we have the capacity to
        assert_eq!(*count.lock().unwrap(), 3);
        advance(Duration::from_secs(3)).await;
        assert!(poll!(runner.as_mut()).is_pending());

        // Send the third message again and check it's ran
        sched_tx
            .send(ScheduleRequest {
                message: 3,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        advance(Duration::from_secs(3)).await;
        assert!(poll!(runner.as_mut()).is_pending());
        assert_eq!(*count.lock().unwrap(), 4);

        let (mut sched_tx, sched_rx) = mpsc::unbounded();
        let mut runner = Box::pin(
            Runner::new(scheduler(sched_rx), 1, |_| {
                DurationalFuture::new(Duration::from_secs(2))
            })
            .for_each(|_| async {}),
        );

        sched_tx
            .send(ScheduleRequest {
                message: 1,
                run_at: Instant::now(),
            })
            .await
            .unwrap();
        assert!(poll!(runner.as_mut()).is_pending());

        // Drop the sender to test that we stop the runner when the requests
        // stream finishes.
        drop(sched_tx);
        assert_eq!(poll!(runner.as_mut()), Poll::Pending);
    }
}
