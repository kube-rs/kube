use crate::{utils::stream_backoff::StreamBackoff, watcher};
use backoff::backoff::Backoff;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{Stream, TryStream};
use pin_project::pin_project;

// grab from private part of tokio
macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}

/// Extension trait for streams of returned by [`watcher`] or [`reflector`]
pub trait WatchStreamExt: Stream {
    /// Apply a [`Backoff`] policy to a [`Stream`] using [`StreamBackoff`]
    fn backoff<B>(self, b: B) -> StreamBackoff<Self, B>
    where
        B: Backoff,
        Self: TryStream + Sized,
    {
        StreamBackoff::new(self, b)
    }

    // This works with bottom setup - but is extremely specific with input/output types
    fn watch_applies<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, false)
    }

    // This works with bottom setup - but is extremely specific with input/output types
    fn watch_touches<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, true)
    }
}
impl<St: ?Sized> WatchStreamExt for St where St: Stream {}

#[pin_project]
/// Stream returned by the [`watch_applies`](super::WatchStreamExt::watch_applies) method.
#[must_use = "streams do nothing unless polled"]
pub struct EventFlatten<St, K> {
    #[pin]
    stream: St,
    delete: bool,
    state: Option<Result<watcher::Event<K>, watcher::Error>>,
}
impl<St: TryStream<Ok = watcher::Event<K>>, K> EventFlatten<St, K> {
    pub(super) fn new(stream: St, delete: bool) -> Self {
        Self {
            stream,
            state: None,
            delete,
        }
    }
}
impl<St, K> Stream for EventFlatten<St, K>
where
    St: Stream<Item = Result<watcher::Event<K>, watcher::Error>>,
{
    type Item = Result<K, watcher::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        loop {
            if let Some(curr) = me.state.take() {
                match curr {
                    Ok(event) => {
                        // drain an individual event as per Event::into_iter_applied
                        match event {
                            watcher::Event::Applied(obj) => {
                                return Poll::Ready(Some(Ok(obj)));
                            }
                            watcher::Event::Deleted(obj) => {
                                // only pass delete events for touches
                                if *me.delete {
                                    return Poll::Ready(Some(Ok(obj)));
                                }
                            }
                            watcher::Event::Restarted(mut reslist) => {
                                if let Some(last) = reslist.pop() {
                                    // store the remainder
                                    *me.state = Some(Ok(watcher::Event::Restarted(reslist)));
                                    return Poll::Ready(Some(Ok(last)));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Poll::Ready(Some(Err(e)));
                    }
                }
            }
            let next = ready!(me.stream.as_mut().poll_next(cx));
            match next {
                Some(event) => {
                    *me.state = Some(event); // continue around loop to extract from it
                }
                None => return Poll::Pending,
            }
        }
    }
}
