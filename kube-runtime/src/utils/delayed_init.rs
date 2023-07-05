#[cfg(test)] use std::sync::RwLockWriteGuard;
use std::{fmt::Debug, task::Poll};

use derivative::Derivative;
use futures::{channel, Future, FutureExt};
use std::sync::RwLock;
use thiserror::Error;
use tracing::trace;

/// The sending counterpart to a [`DelayedInit`]
pub struct Initializer<T>(channel::oneshot::Sender<T>);
impl<T> Initializer<T> {
    /// Sends `value` to the linked [`DelayedInit`].
    pub fn init(self, value: T) {
        // oneshot::Sender::send fails if no recipients remain, this is not really a relevant
        // case to signal for our use case
        let _ = self.0.send(value);
    }
}
impl<T> Debug for Initializer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("delayed_init::Initializer")
            .finish_non_exhaustive()
    }
}

/// A value that must be initialized by an external writer
///
/// Can be considered equivalent to a [`channel::oneshot`] channel, except for that
/// the value produced is retained for subsequent calls to [`Self::get`].
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DelayedInit<T> {
    state: RwLock<ReceiverState<T>>,
    // A test-only hook to let us create artificial race conditions
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    #[derivative(Debug = "ignore")]
    test_hook_start_of_slow_path: Box<dyn Fn(&mut RwLockWriteGuard<ReceiverState<T>>) + Send + Sync>,
}
#[derive(Debug)]
enum ReceiverState<T> {
    Waiting(channel::oneshot::Receiver<T>),
    Ready(Result<T, InitDropped>),
}
impl<T> DelayedInit<T> {
    /// Returns an empty `DelayedInit` that has no value, along with a linked [`Initializer`]
    #[must_use]
    pub fn new() -> (Initializer<T>, Self) {
        let (tx, rx) = channel::oneshot::channel();
        (Initializer(tx), DelayedInit {
            state: RwLock::new(ReceiverState::Waiting(rx)),
            #[cfg(test)]
            test_hook_start_of_slow_path: Box::new(|_| ()),
        })
    }
}
impl<T: Clone + Send + Sync> DelayedInit<T> {
    /// Wait for the value to be available and then return it
    ///
    /// Calling `get` again if a value has already been returned is guaranteed to return (a clone of)
    /// the same value.
    ///
    /// # Errors
    ///
    /// Fails if the associated [`Initializer`] has been dropped before calling [`Initializer::init`].
    pub async fn get(&self) -> Result<T, InitDropped> {
        Get(self).await
    }
}

// Using a manually implemented future because we don't want to hold the lock across poll calls
// since that would mean that an unpolled writer would stall all other tasks from being able to poll it
pub struct Get<'a, T>(&'a DelayedInit<T>);
impl<'a, T> Future for Get<'a, T>
where
    T: Clone,
{
    type Output = Result<T, InitDropped>;

    #[tracing::instrument(name = "DelayedInit::get", level = "trace", skip(self, cx))]
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let read_lock = self.0.state.read().unwrap();
        if let ReceiverState::Ready(v) = &*read_lock {
            trace!("using fast path, value is already ready");
            Poll::Ready(v.clone())
        } else {
            trace!("using slow path, need to wait for the channel");
            // IMPORTANT: Make sure that the optimistic read lock has been released already
            drop(read_lock);
            let mut state = self.0.state.write().unwrap();
            trace!("got write lock");
            #[cfg(test)]
            (self.0.test_hook_start_of_slow_path)(&mut state);
            match &mut *state {
                ReceiverState::Waiting(rx) => {
                    trace!("channel still active, polling");
                    if let Poll::Ready(value) = rx.poll_unpin(cx).map_err(|_| InitDropped) {
                        trace!("got value on slow path, memoizing");
                        *state = ReceiverState::Ready(value.clone());
                        Poll::Ready(value)
                    } else {
                        trace!("channel is still pending");
                        Poll::Pending
                    }
                }
                ReceiverState::Ready(v) => {
                    trace!("slow path but value was already initialized, another writer already initialized");
                    Poll::Ready(v.clone())
                }
            }
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("initializer was dropped before value was initialized")]
pub struct InitDropped;

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        task::Poll,
    };

    use futures::{pin_mut, poll};
    use tracing::Level;
    use tracing_subscriber::util::SubscriberInitExt;

    use crate::utils::delayed_init::ReceiverState;

    use super::DelayedInit;

    fn setup_tracing() -> tracing::dispatcher::DefaultGuard {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .with_test_writer()
            .finish()
            .set_default()
    }

    #[tokio::test]
    async fn must_allow_single_reader() {
        let _tracing = setup_tracing();
        let (tx, rx) = DelayedInit::<u8>::new();
        let get1 = rx.get();
        pin_mut!(get1);
        assert_eq!(poll!(get1.as_mut()), Poll::Pending);
        tx.init(1);
        assert_eq!(poll!(get1), Poll::Ready(Ok(1)));
    }

    #[tokio::test]
    async fn must_allow_concurrent_readers_while_waiting() {
        let _tracing = setup_tracing();
        let (tx, rx) = DelayedInit::<u8>::new();
        let get1 = rx.get();
        let get2 = rx.get();
        let get3 = rx.get();
        pin_mut!(get1, get2, get3);
        assert_eq!(poll!(get1.as_mut()), Poll::Pending);
        assert_eq!(poll!(get2.as_mut()), Poll::Pending);
        assert_eq!(poll!(get3.as_mut()), Poll::Pending);
        tx.init(1);
        assert_eq!(poll!(get1), Poll::Ready(Ok(1)));
        assert_eq!(poll!(get2), Poll::Ready(Ok(1)));
        assert_eq!(poll!(get3), Poll::Ready(Ok(1)));
    }

    #[tokio::test]
    async fn must_allow_reading_after_init() {
        let _tracing = setup_tracing();
        let (tx, rx) = DelayedInit::<u8>::new();
        let get1 = rx.get();
        pin_mut!(get1);
        assert_eq!(poll!(get1.as_mut()), Poll::Pending);
        tx.init(1);
        assert_eq!(poll!(get1), Poll::Ready(Ok(1)));
        assert_eq!(rx.get().await, Ok(1));
        assert_eq!(rx.get().await, Ok(1));
    }

    #[tokio::test]
    async fn must_allow_concurrent_readers_in_any_order() {
        let _tracing = setup_tracing();
        let (tx, rx) = DelayedInit::<u8>::new();
        let get1 = rx.get();
        let get2 = rx.get();
        let get3 = rx.get();
        pin_mut!(get1, get2, get3);
        assert_eq!(poll!(get1.as_mut()), Poll::Pending);
        assert_eq!(poll!(get2.as_mut()), Poll::Pending);
        assert_eq!(poll!(get3.as_mut()), Poll::Pending);
        tx.init(1);
        assert_eq!(poll!(get3), Poll::Ready(Ok(1)));
        assert_eq!(poll!(get2), Poll::Ready(Ok(1)));
        assert_eq!(poll!(get1), Poll::Ready(Ok(1)));
    }

    #[tokio::test]
    async fn must_work_despite_writer_race() {
        let _tracing = setup_tracing();
        let (_tx, mut rx) = DelayedInit::<u8>::new();
        let slow_path_calls = Arc::new(Mutex::new(0));
        let slow_path_calls2 = slow_path_calls.clone();
        // This emulates two racing get() calls, where a fake get() memoizes a value while the true get()
        // is waiting to upgrade its read lock to a write lock.
        rx.test_hook_start_of_slow_path = Box::new(move |state| {
            *slow_path_calls2.lock().unwrap() += 1;
            **state = ReceiverState::Ready(Ok(1));
        });
        assert_eq!(rx.get().await, Ok(1));
        assert_eq!(rx.get().await, Ok(1));
        assert_eq!(rx.get().await, Ok(1));
        assert_eq!(*slow_path_calls.lock().unwrap(), 1);
    }
}
