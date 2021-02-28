use futures::{Future, FutureExt, Stream};
use std::{
    collections::HashMap,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};

/// Variant of [`tokio_stream::StreamMap`](https://docs.rs/tokio-stream/0.1.3/tokio_stream/struct.StreamMap.html)
/// that only runs [`Future`]s, and uses a [`HashMap`] as the backing store, giving (amortized) O(1) insertion
/// and membership checks.
///
/// Just like for `StreamMap`'s `S`, `F` must be [`Unpin`], since [`HashMap`] is free to move
/// entries as it pleases (for example: resizing the backing array).
///
/// NOTE: Contrary to `StreamMap`, `FutureHashMap` does *not* try to be fair. The polling order
/// is arbitrary, but generally stable while the future set is (although this should not be relied on!).
#[derive(Debug)]
pub struct FutureHashMap<K, F> {
    futures: HashMap<K, F>,
}

impl<K, F> Default for FutureHashMap<K, F> {
    fn default() -> Self {
        Self {
            futures: HashMap::new(),
        }
    }
}

impl<K, F> FutureHashMap<K, F>
where
    K: Hash + Eq,
{
    /// Inserts `future` into the key `key`, returning the old future if there was one
    pub fn insert(&mut self, key: K, future: F) -> Option<F> {
        self.futures.insert(key, future)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.futures.contains_key(key)
    }
}

impl<K, F> Stream for FutureHashMap<K, F>
where
    K: Hash + Clone + Eq,
    F: Future + Unpin,
    Self: Unpin,
{
    type Item = F::Output;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let key_and_msg =
            self.as_mut()
                .futures
                .iter_mut()
                .find_map(|(key, future)| match future.poll_unpin(cx) {
                    Poll::Ready(msg) => Some((key.clone(), msg)),
                    Poll::Pending => None,
                });
        //dbg!((key_and_msg.is_some(), &self.futures.len()));
        match key_and_msg {
            Some((key, msg)) => {
                self.as_mut().futures.remove(&key);
                Poll::Ready(Some(msg))
            }
            None if self.futures.is_empty() => Poll::Ready(None),
            None => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use super::FutureHashMap;
    use futures::{channel::mpsc, future, poll, StreamExt};

    #[tokio::test]
    async fn fhm_should_forward_all_values_and_shut_down() {
        let mut fhm = FutureHashMap::default();
        let count = 100;
        for i in 0..count {
            fhm.insert(i, future::ready(i));
        }
        let mut values = fhm.collect::<Vec<u16>>().await;
        values.sort_unstable();
        assert_eq!(values, (0..count).collect::<Vec<u16>>());
    }

    #[tokio::test]
    async fn fhm_should_stay_alive_until_all_sources_finish() {
        let mut fhm = FutureHashMap::default();
        let (tx0, mut rx0) = mpsc::unbounded::<()>();
        let (tx1, mut rx1) = mpsc::unbounded::<()>();
        fhm.insert(0, rx0.next());
        fhm.insert(1, rx1.next());
        assert_eq!(poll!(fhm.next()), Poll::Pending);
        drop(tx0);
        assert_eq!(poll!(fhm.next()), Poll::Ready(Some(None)));
        assert_eq!(poll!(fhm.next()), Poll::Pending);
        drop(tx1);
        assert_eq!(poll!(fhm.next()), Poll::Ready(Some(None)));
        assert_eq!(poll!(fhm.next()), Poll::Ready(None));
    }
}
