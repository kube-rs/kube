use futures::{Future, Stream, StreamExt};
use std::{
    collections::HashMap,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};

/// Variant of [`tokio::stream::StreamMap`] that uses a [`HashMap`] as the backing store,
/// giving O(1) insertion and membership checks.
///
/// Just like for `StreamMap`, `S` must be [`Unpin`], since [`HashMap`] is free to move
/// entries as it pleases (for example: resizing the backing array).
///
/// NOTE: Contrary to `StreamMap`, `StreamHashMap` does *not* try to be fair. The polling order
/// is arbitrary, but generally stable while the stream set is (although this should not be relied on!).
pub struct StreamHashMap<K, S> {
    streams: HashMap<K, S>,
}

impl<K, S> Default for StreamHashMap<K, S> {
    fn default() -> Self {
        Self {
            streams: HashMap::new(),
        }
    }
}

impl<K, S> StreamHashMap<K, S>
where
    K: Hash + Eq,
{
    /// Inserts `stream` into the key `key`, returning the old stream if there was one
    pub fn insert(&mut self, key: K, stream: S) -> Option<S> {
        self.streams.insert(key, stream)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.streams.contains_key(key)
    }
}

impl<K, F> StreamHashMap<K, futures::stream::Once<F>>
where
    K: Hash + Eq,
    F: Future,
{
    pub fn insert_future(&mut self, key: K, future: F) -> bool {
        self.insert(key, futures::stream::once(future)).is_some()
    }
}

impl<K, S> Stream for StreamHashMap<K, S>
where
    K: Hash + Clone + Eq,
    S: Stream + Unpin,
    Self: Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut finished_keys = Vec::new();
        for (key, stream) in &mut self.as_mut().streams {
            match stream.poll_next_unpin(cx) {
                Poll::Ready(Some(x)) => return Poll::Ready(Some(x)),
                // Can't remove the finished keys immediately, since that would
                // invalidate the iterator
                Poll::Ready(None) => finished_keys.push(key.clone()),
                Poll::Pending => {}
            }
        }
        for key in finished_keys {
            self.as_mut().streams.remove(&key);
        }
        if self.streams.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use super::StreamHashMap;
    use futures::{channel::mpsc, poll, stream, StreamExt};

    #[tokio::test]
    async fn shm_should_forward_all_values_and_shut_down() {
        let mut shm = StreamHashMap::default();
        let chunk_count = 10;
        let chunk_size = 10;
        for i in 0..chunk_count {
            shm.insert(i, stream::iter(0..chunk_size).map(move |x| x + i * chunk_size));
        }
        let mut values = shm.collect::<Vec<u16>>().await;
        values.sort();
        assert_eq!(values, (0..chunk_count * chunk_size).collect::<Vec<u16>>());
    }

    #[tokio::test]
    async fn shm_should_stay_alive_until_all_sources_finish() {
        let mut shm = StreamHashMap::default();
        let (tx0, rx0) = mpsc::unbounded::<()>();
        let (tx1, rx1) = mpsc::unbounded::<()>();
        shm.insert(0, rx0);
        shm.insert(1, rx1);
        assert_eq!(poll!(shm.next()), Poll::Pending);
        drop(tx0);
        assert_eq!(poll!(shm.next()), Poll::Pending);
        drop(tx1);
        assert_eq!(poll!(shm.next()), Poll::Ready(None))
    }
}
