use std::{
    error::Error as StdError,
    fmt,
    pin::Pin,
    task::{ready, Context, Poll},
};

use bytes::Bytes;
use futures::{stream::Stream, TryStreamExt};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, BodyStream};

/// A request body.
pub struct Body {
    kind: Kind,
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body").finish()
    }
}

enum Kind {
    Once(Option<Bytes>),
    Wrap(UnsyncBoxBody<Bytes, Box<dyn StdError + Send + Sync>>),
}

impl Body {
    fn new(kind: Kind) -> Self {
        Body { kind }
    }

    /// Create an empty body
    pub fn empty() -> Self {
        Self::new(Kind::Once(None))
    }

    pub(crate) fn wrap_body<B>(body: B) -> Self
    where
        B: HttpBody<Data = Bytes> + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Body::new(Kind::Wrap(body.map_err(Into::into).boxed_unsync()))
    }

    /// Collect all the pieces of the body
    pub async fn collect_bytes(self) -> Result<Bytes, crate::Error> {
        Ok(<Self as BodyExt>::collect(self).await?.to_bytes())
    }

    pub(crate) fn into_data_stream(
        self,
    ) -> impl Stream<Item = Result<<Self as HttpBody>::Data, <Self as HttpBody>::Error>> {
        Box::pin(BodyStream::new(self).try_filter_map(|frame| async { Ok(frame.into_data().ok()) }))
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        if bytes.is_empty() {
            Self::empty()
        } else {
            Self::new(Kind::Once(Some(bytes)))
        }
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Self {
        Self::from(Bytes::from(vec))
    }
}

impl HttpBody for Body {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.kind {
            Kind::Once(ref mut val) => {
                if let Some(data) = val.take() {
                    Poll::Ready(Some(Ok(Frame::data(data))))
                } else {
                    Poll::Ready(None)
                }
            }
            Kind::Wrap(ref mut stream) => Poll::Ready(
                ready!(Pin::new(stream).poll_frame(cx))
                    .map(|opt_chunk| opt_chunk.map_err(crate::Error::Service)),
            ),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            Kind::Once(Some(bytes)) => SizeHint::with_exact(bytes.len() as u64),
            Kind::Once(None) => SizeHint::with_exact(0),
            Kind::Wrap(body) => body.size_hint(),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            Kind::Once(Some(bytes)) => bytes.is_empty(),
            Kind::Once(None) => true,
            Kind::Wrap(body) => body.is_end_stream(),
        }
    }
}
