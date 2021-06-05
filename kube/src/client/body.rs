use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::stream::Stream;
use http_body::Body;
use pin_project::pin_project;

// Wrap `http_body::Body` to implement `Stream`.
#[pin_project]
pub struct IntoStream<B> {
    #[pin]
    body: B,
}

impl<B> IntoStream<B> {
    pub(crate) fn new(body: B) -> Self {
        Self { body }
    }
}

impl<B> Stream for IntoStream<B>
where
    B: Body,
{
    type Item = Result<B::Data, B::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().body.poll_data(cx)
    }
}

pub trait BodyStreamExt: Body {
    fn into_stream(self) -> IntoStream<Self>
    where
        Self: Sized,
    {
        IntoStream::new(self)
    }
}

impl<T> BodyStreamExt for T where T: Body {}
