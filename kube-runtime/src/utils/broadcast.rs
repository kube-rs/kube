use ahash::HashMap;
use futures::{channel::mpsc, stream, SinkExt, Stream, StreamExt};

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct SubscriberId(u64);

/// A channel that broadcasts values to multiple subscribers.
///
/// Compared to [`tokio::sync::broadcast`], this backpressures the sender when the buffer is full,
/// rather than dropping values.
#[derive(Debug)]
pub(crate) struct Broadcaster<T> {
    subscriber_txes: HashMap<SubscriberId, mpsc::Sender<T>>,
    next_id: SubscriberId,
    buffer_size: usize,
}

impl<T> Broadcaster<T>
where
    T: Clone,
{
    pub fn new(buffer_size: usize) -> Self {
        Self {
            subscriber_txes: HashMap::default(),
            next_id: SubscriberId(0),
            buffer_size,
        }
    }

    /// Sends `value` to all subscribers.
    ///
    /// The future may wait if any subscriber does not have room to buffer the value.
    ///
    /// Closed subscribers will be unsubscribed.
    pub async fn send(&mut self, value: T) {
        let closed_subscribers = stream::iter(
            // TODO: This should be able to be a &mut, but that causes a weird lifetime error that somehow gets linked to the call site
            self.subscriber_txes.clone(),
        )
        .flat_map_unordered(None, |(sub_id, mut tx)| {
            let value = value.clone();
            Box::pin(stream::once(async move {
                match tx.send(value).await {
                    // Subscriber is still open
                    Ok(()) => None,
                    // Subscriber is closed, schedule for unsubscribing
                    Err(_) => Some(sub_id),
                }
            }))
        })
        .filter_map(|x: Option<SubscriberId>| async move { x })
        .collect::<Vec<_>>()
        .await;
        for closed_sub in closed_subscribers {
            self.subscriber_txes.remove(&closed_sub);
        }
    }

    pub fn subscribe(&mut self) -> impl Stream<Item = T> {
        // Currently we allocate a buffer per subscriber, but it is configured over the whole stream
        // in order to give room to move to a shared buffer implementation later on.
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let id = self.next_id;
        self.next_id = SubscriberId(id.0 + 1);
        self.subscriber_txes.insert(id, tx);
        rx
    }
}

#[cfg(test)]
mod tests {
    use std::{pin::pin, time::Duration};

    use futures::{future, poll, FutureExt, StreamExt};
    use tokio::time::timeout;

    use super::Broadcaster;

    #[tokio::test]
    async fn test_regular_usage() {
        let mut broadcaster = Broadcaster::<u8>::new(1);
        let sent = (0..20).collect::<Vec<_>>();
        let subscribers = (0..10)
            .map(|_| {
                broadcaster
                    .subscribe()
                    .collect::<Vec<_>>()
                    .map(|received| assert_eq!(sent, received))
                    .boxed()
            })
            .collect::<Vec<_>>();
        let producer = async {
            for i in &sent {
                broadcaster.send(*i).await;
            }
            drop(broadcaster);
        }
        .boxed();
        timeout(
            Duration::from_secs(1),
            future::join_all(subscribers.into_iter().chain([producer])),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_backpressure() {
        timeout(Duration::from_secs(1), async {
            let mut broadcaster = Broadcaster::<u8>::new(1);
            let mut sub_1 = broadcaster.subscribe();
            let mut sub_2 = broadcaster.subscribe();

            broadcaster.send(1).await;
            assert_eq!(Some(1), sub_1.next().await);
            // No read on sub_2

            let mut send_2 = pin!(broadcaster.send(2));
            // send_2 will be pending due to 1 not having been read yet from sub_2 (so sub_2's buffer is full)
            assert!(poll!(send_2.as_mut()).is_pending());
            // sub_1 has buffer space, so it will see 2 immediately
            assert_eq!(Some(2), sub_1.next().await);
            // read 1 from sub_2 to make room
            assert_eq!(Some(1), sub_2.next().await);
            // send_2 should now be able to complete since sub_2 has room
            send_2.await;
            assert_eq!(Some(2), sub_2.next().await);
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut broadcaster = Broadcaster::<u8>::new(1);
        let sub_1 = broadcaster.subscribe();
        let sub_2 = broadcaster.subscribe();

        let subscriber = sub_1.collect::<Vec<_>>();
        let producer = async {
            for i in 0..10 {
                broadcaster.send(i).await
            }
            drop(broadcaster);
        };

        // unsubscribe sub_2
        drop(sub_2);
        timeout(Duration::from_secs(1), future::join(subscriber, producer))
            .await
            .unwrap();
    }
}
