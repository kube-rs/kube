use bytes::Bytes;
use futures::{ready, stream::MapErr, Future, Stream, StreamExt, TryStreamExt};
use http::Response;
use hyper::Body;
use serde::de::DeserializeOwned;
use snafu::{ResultExt, Snafu};
use std::{convert::Infallible, marker::PhantomData, task::Poll};
use tokio_util::{
    codec::{FramedRead, LinesCodec, LinesCodecError},
    io::StreamReader,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("read failed: {}", source))]
    ReadFailed { source: LinesCodecError },
    #[snafu(display("deserialize failed: {}", source))]
    DeserializeFailed { source: serde_json::Error },
}

pub struct DecodeStream<K> {
    tpe: PhantomData<*const K>,
    body: Option<Body>,
}

impl<K> Future for DecodeStream<K> {
    type Output = Result<DecodeStreamStream<K>, Infallible>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        Poll::Ready(Ok(DecodeStreamStream {
            tpe: self.tpe,
            body: FramedRead::new(
                StreamReader::new(
                    self.body
                        .take()
                        .expect("DecodeStream may not be polled again after resolving")
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
                ),
                LinesCodec::new(),
            ),
        }))
    }
}

impl<K> From<Response<Body>> for DecodeStream<K> {
    fn from(res: Response<Body>) -> Self {
        Self {
            tpe: PhantomData,
            body: Some(res.into_body()),
        }
    }
}

pub struct DecodeStreamStream<K> {
    tpe: PhantomData<*const K>,
    #[allow(clippy::type_complexity)]
    body: FramedRead<
        StreamReader<MapErr<Body, fn(hyper::Error) -> std::io::Error>, Bytes>,
        LinesCodec,
    >,
}

impl<K: DeserializeOwned> Stream for DecodeStreamStream<K> {
    type Item = Result<K, Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.body.poll_next_unpin(cx)) {
            Some(frame) => Poll::Ready(Some(read_frame(frame))),
            None => Poll::Ready(None),
        }
    }
}

fn read_frame<K: DeserializeOwned>(frame: Result<String, LinesCodecError>) -> Result<K, Error> {
    serde_json::from_str(&frame.context(ReadFailed)?).context(DeserializeFailed)
}
