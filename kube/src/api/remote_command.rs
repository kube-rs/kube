use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Status;

use async_tungstenite::{tokio::ConnectStream, tungstenite as ws, WebSocketStream};
use futures::{
    future::Either::{Left, Right},
    SinkExt, Stream, StreamExt, TryStreamExt,
};
use futures_util::future::select;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream};
use tokio_util::codec;

// Internal state of an attached process
struct AttachedProcessState {
    waker: Option<Waker>,
    finished: bool,
    status: Option<Status>,
    stdin_writer: Option<DuplexStream>,
    stdout_reader: Option<DuplexStream>,
    stderr_reader: Option<DuplexStream>,
}

/// Represents an attached process in a container for [`attach`] and [`exec`].
///
/// Resolves when the connection terminates with an optional [`Status`].
/// Provides access to `stdin`, `stdout`, and `stderr` if attached.
///
/// [`attach`]: crate::Api::attach
/// [`exec`]: crate::Api::exec
/// [`Status`]: k8s_openapi::apimachinery::pkg::apis::meta::v1::Status
pub struct AttachedProcess {
    has_stdin: bool,
    has_stdout: bool,
    has_stderr: bool,
    state: Arc<Mutex<AttachedProcessState>>,
}

impl AttachedProcess {
    pub(crate) fn new(
        stream: WebSocketStream<ConnectStream>,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Self {
        // To simplify the implementation, always create a pipe for stdin.
        // The caller does not have access to it unless they had requested.
        // REVIEW Make internal buffer size configurable?
        let (stdin_writer, stdin_reader) = tokio::io::duplex(1024);
        let (stdout_writer, stdout_reader) = if stdout {
            let (w, r) = tokio::io::duplex(1024);
            (Some(w), Some(r))
        } else {
            (None, None)
        };
        let (stderr_writer, stderr_reader) = if stderr {
            let (w, r) = tokio::io::duplex(1024);
            (Some(w), Some(r))
        } else {
            (None, None)
        };

        let state = Arc::new(Mutex::new(AttachedProcessState {
            waker: None,
            finished: false,
            status: None,
            stdin_writer: Some(stdin_writer),
            stdout_reader,
            stderr_reader,
        }));
        let shared_state = state.clone();
        tokio::spawn(async move {
            let status = start_message_loop(stream, stdin_reader, stdout_writer, stderr_writer).await;

            let mut shared = shared_state.lock().unwrap();
            shared.finished = true;
            shared.status = status;
            if let Some(waker) = shared.waker.take() {
                waker.wake()
            }
        });

        AttachedProcess {
            has_stdin: stdin,
            has_stdout: stdout,
            has_stderr: stderr,
            state,
        }
    }

    /// Async writer to stdin.
    /// ```ignore
    /// let mut stdin_writer = attached.stdin().unwrap();
    /// stdin_writer.write(b"foo\n").await?;
    /// ```
    /// Only available if [`AttachParams`](super::AttachParams) had `stdin`.
    pub fn stdin(&mut self) -> Option<impl AsyncWrite + Unpin> {
        if !self.has_stdin {
            return None;
        }

        let mut state = self.state.lock().unwrap();
        if let Some(writer) = state.stdin_writer.take() {
            Some(writer)
        } else {
            None
        }
    }

    /// Stream of stdout outputs.
    /// ```ignore
    /// let mut stdout_stream = attached.stdout().unwrap();
    /// let next_stdout = stdout_stream.next().await?;
    /// ```
    /// Only available if [`AttachParams`](super::AttachParams) had `stdout`.
    pub fn stdout(&mut self) -> Option<impl Stream<Item = Result<Vec<u8>, std::io::Error>>> {
        if !self.has_stdout {
            return None;
        }

        let mut state = self.state.lock().unwrap();
        if let Some(reader) = state.stdout_reader.take() {
            Some(into_bytes_stream(reader))
        } else {
            None
        }
    }

    /// Stream of stderr outputs.
    /// ```ignore
    /// let mut stderr_stream = attached.stderr().unwrap();
    /// let next_stderr = stderr_stream.next().await?;
    /// ```
    /// Only available if [`AttachParams`](super::AttachParams) had `stderr`.
    pub fn stderr(&mut self) -> Option<impl Stream<Item = Result<Vec<u8>, std::io::Error>>> {
        if !self.has_stderr {
            return None;
        }

        let mut state = self.state.lock().unwrap();
        if let Some(reader) = state.stderr_reader.take() {
            Some(into_bytes_stream(reader))
        } else {
            None
        }
    }
}

impl Future for AttachedProcess {
    type Output = Option<Status>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if state.finished {
            Poll::Ready(state.status.take())
        } else {
            // Update waker if necessary
            if let Some(waker) = &state.waker {
                if waker.will_wake(cx.waker()) {
                    return Poll::Pending;
                }
            }

            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

const STDIN_CHANNEL: u8 = 0;
const STDOUT_CHANNEL: u8 = 1;
const STDERR_CHANNEL: u8 = 2;
// status channel receives `Status` object on exit.
const STATUS_CHANNEL: u8 = 3;
// const RESIZE_CHANNEL: u8 = 4;

async fn start_message_loop(
    stream: WebSocketStream<ConnectStream>,
    stdin: impl AsyncRead + Unpin,
    mut stdout: Option<impl AsyncWrite + Unpin>,
    mut stderr: Option<impl AsyncWrite + Unpin>,
) -> Option<Status> {
    let mut stdin_stream = into_bytes_stream(stdin);
    let (mut server_send, raw_server_recv) = stream.split();
    // Work with filtered messages to reduce noise.
    let mut server_recv = raw_server_recv.filter_map(filter_message).boxed();
    let mut server_msg = server_recv.next();
    let mut next_stdin = stdin_stream.next();
    let mut status: Option<Status> = None;

    loop {
        match select(server_msg, next_stdin).await {
            // from server
            Left((Some(message), p_next_stdin)) => {
                match message {
                    Ok(Message::Stdout(bin)) => {
                        if let Some(stdout) = stdout.as_mut() {
                            stdout
                                .write_all(&bin[1..])
                                .await
                                .expect("stdout pipe is writable");
                        }
                    }

                    Ok(Message::Stderr(bin)) => {
                        if let Some(stderr) = stderr.as_mut() {
                            stderr
                                .write_all(&bin[1..])
                                .await
                                .expect("stderr pipe is writable");
                        }
                    }

                    Ok(Message::Status(bin)) => {
                        if let Ok(s) = serde_json::from_slice::<Status>(&bin[1..]) {
                            status = Some(s);
                        }
                    }

                    // Fatal error
                    Err(err) => {
                        panic!("AttachedProcess: fatal WebSocket error: {:?}", err);
                    }
                }
                server_msg = server_recv.next();
                next_stdin = p_next_stdin;
            }

            Left((None, _)) => {
                // Connection closed properly
                break;
            }

            // from stdin
            Right((Some(Ok(bytes)), p_server_msg)) => {
                if !bytes.is_empty() {
                    let mut vec = Vec::with_capacity(bytes.len() + 1);
                    vec.push(STDIN_CHANNEL);
                    vec.extend_from_slice(&bytes[..]);
                    server_send
                        .send(ws::Message::binary(vec))
                        .await
                        .expect("send stdin");
                }
                server_msg = p_server_msg;
                next_stdin = stdin_stream.next();
            }

            Right((Some(Err(err)), _)) => {
                server_send.close().await.expect("send close message");
                panic!("AttachedProcess: failed to read from stdin pipe: {:?}", err);
            }

            Right((None, _)) => {
                // Stdin closed (writer half dropped).
                // Let the server know and disconnect.
                // REVIEW warn?
                server_send.close().await.expect("send close message");
                break;
            }
        }
    }

    status
}

fn into_bytes_stream<R: AsyncRead>(reader: R) -> impl Stream<Item = Result<Vec<u8>, std::io::Error>> {
    codec::FramedRead::new(reader, codec::BytesCodec::new()).map_ok(|bs| bs.freeze()[..].to_vec())
}

/// Channeled messages from the server.
enum Message {
    /// To Stdout channel (1)
    Stdout(Vec<u8>),
    /// To stderr channel (2)
    Stderr(Vec<u8>),
    /// To error/status channel (3)
    Status(Vec<u8>),
}

// Filter to reduce all the possible WebSocket messages into a few we expect to receive.
async fn filter_message(wsm: Result<ws::Message, ws::Error>) -> Option<Result<Message, ws::Error>> {
    match wsm {
        // The protocol only sends binary frames.
        // Message of size 1 (only channel number) is sent on connection.
        Ok(ws::Message::Binary(bin)) if bin.len() > 1 => match bin[0] {
            STDOUT_CHANNEL => Some(Ok(Message::Stdout(bin))),
            STDERR_CHANNEL => Some(Ok(Message::Stderr(bin))),
            STATUS_CHANNEL => Some(Ok(Message::Status(bin))),
            // We don't receive messages to stdin and resize channels.
            _ => None,
        },
        // Ignore any other message types.
        // We can ignore close message because the server never sends anything special.
        // The connection terminates on `None`.
        Ok(_) => None,
        // Fatal errors. `WebSocketStream` turns `ConnectionClosed` and `AlreadyClosed` into `None`.
        // So these are unrecoverables.
        Err(err) => Some(Err(err)),
    }
}
