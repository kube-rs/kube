//! Single-value decoder

use bytes::{Buf, Bytes};
use futures::{ready, Future, StreamExt};
use http::Response;
use hyper::Body;
use serde::de::DeserializeOwned;
use snafu::{ResultExt, Snafu};
use std::{io::Read, marker::PhantomData, task::Poll};

#[derive(Debug, Snafu)]
#[allow(missing_docs)]
/// Failed to decode body
pub enum Error {
    /// Failed to read body
    #[snafu(display("read failed: {}", source))]
    ReadFailed { source: hyper::Error },
    /// Failed to deserialize body
    #[snafu(display("deserialize failed: {}", source))]
    DeserializeFailed { source: serde_json::Error },
}

/// Decode a single JSON value
pub struct DecodeSingle<K> {
    tpe: PhantomData<fn() -> K>,
    chunks: Vec<Bytes>,
    body: Body,
}

impl<K> From<Response<Body>> for DecodeSingle<K> {
    fn from(res: Response<Body>) -> Self {
        Self {
            tpe: PhantomData,
            chunks: Vec::new(),
            body: res.into_body(),
        }
    }
}

impl<K: DeserializeOwned> Future for DecodeSingle<K> {
    type Output = Result<K, Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            break match ready!(self.body.poll_next_unpin(cx)) {
                Some(Ok(chunk)) => {
                    self.chunks.push(chunk);
                    continue;
                }
                Some(Err(err)) => Poll::Ready(Err(err).context(ReadFailed)),
                None => Poll::Ready(
                    serde_json::from_reader(BytesVecCursor::from(std::mem::take(&mut self.chunks)))
                        .context(DeserializeFailed),
                ),
            };
        }
    }
}

struct BytesVecCursor {
    cur_chunk: bytes::buf::Reader<Bytes>,
    chunks: std::vec::IntoIter<Bytes>,
}

impl Read for BytesVecCursor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            break Ok(match self.cur_chunk.read(buf)? {
                0 => match self.chunks.next() {
                    Some(chunk) => {
                        self.cur_chunk = chunk.reader();
                        continue;
                    }
                    None => 0,
                },
                n => n,
            });
        }
    }
}

impl From<Vec<Bytes>> for BytesVecCursor {
    fn from(vec: Vec<Bytes>) -> Self {
        BytesVecCursor {
            cur_chunk: Bytes::new().reader(),
            chunks: vec.into_iter(),
        }
    }
}
