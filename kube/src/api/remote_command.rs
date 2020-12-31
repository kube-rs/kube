use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Status;

use async_tungstenite::{
    tokio::ConnectStream,
    tungstenite::{self as ws, Message},
    WebSocketStream,
};
use futures::{future::Either, SinkExt, Stream, StreamExt, TryStreamExt};
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

/// Represents an attached process in a container for `attach` and `exec`.
///
/// Provides access to stdin/stdout/stderr if attached.
/// Resolves when the connection terminates with an optional [`Status`].
///
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
            let status = start(stream, stdin_reader, stdout_writer, stderr_writer)
                .await
                .unwrap();

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

    /// Async writer to write to stdin of the attached process.
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

    /// Stream of outputs from stdout of the attached process.
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

    /// Stream of outputs from stderr of the attached process.
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
const ERROR_CHANNEL: u8 = 3;
const RESIZE_CHANNEL: u8 = 4;

async fn start(
    stream: WebSocketStream<ConnectStream>,
    stdin: impl AsyncRead + Unpin,
    mut stdout: Option<impl AsyncWrite + Unpin>,
    mut stderr: Option<impl AsyncWrite + Unpin>,
) -> Result<Option<Status>, std::io::Error> {
    let mut stdin_stream = into_bytes_stream(stdin);
    let (mut server_send, mut server_recv) = stream.split();
    let mut server_msg = server_recv.next();
    let mut next_stdin = stdin_stream.next();
    let mut status: Option<Status> = None;
    loop {
        match select(server_msg, next_stdin).await {
            Either::Left((message, p_next_stdin)) => {
                match message {
                    Some(Ok(Message::Binary(bin))) if !bin.is_empty() => {
                        // Write to appropriate channel
                        match bin[0] {
                            // stdin
                            STDIN_CHANNEL => {}
                            // stdout
                            STDOUT_CHANNEL => {
                                if let Some(stdout) = stdout.as_mut() {
                                    stdout.write_all(&bin[1..]).await?;
                                }
                            }
                            // stderr
                            STDERR_CHANNEL => {
                                if let Some(stderr) = stderr.as_mut() {
                                    stderr.write_all(&bin[1..]).await?;
                                }
                            }
                            // status
                            ERROR_CHANNEL => {
                                if let Ok(s) = serde_json::from_slice::<Status>(&bin[1..]) {
                                    status = Some(s);
                                }
                            }
                            // resize?
                            RESIZE_CHANNEL => {}
                            _ => {}
                        }
                    }
                    // Ignore empty binary message.
                    // Message of length 1 (only channel number) is sent on connection.
                    Some(Ok(Message::Binary(_))) => {}

                    // Ignore anything else.
                    // The protocol we use never sends text frame.
                    Some(Ok(Message::Text(_))) => {}
                    Some(Ok(Message::Ping(_))) => {}
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) => {
                        // Connection will terminate when None is received.
                    }

                    Some(Err(ws::Error::ConnectionClosed)) => {
                        // not actually an error
                        break;
                    }

                    Some(Err(_err)) => {
                        // TODO Log and clean up
                        break;
                    }

                    None => {
                        // Connection closed
                        break;
                    }
                }
                server_msg = server_recv.next();
                next_stdin = p_next_stdin;
            }

            Either::Right((input, p_server_msg)) => {
                match input {
                    Some(Ok(bytes)) if !bytes.is_empty() => {
                        let mut vec = Vec::with_capacity(bytes.len() + 1);
                        vec.push(STDIN_CHANNEL);
                        vec.extend_from_slice(&bytes[..]);
                        server_send.send(ws::Message::binary(vec)).await.unwrap();
                    }

                    Some(Ok(_)) => {}

                    Some(Err(_)) => {
                        // TODO Handle error?
                    }

                    None => {
                        // Stdin closed
                    }
                }
                server_msg = p_server_msg;
                next_stdin = stdin_stream.next();
            }
        }
    }

    Ok(status)
}

fn into_bytes_stream<R: AsyncRead>(reader: R) -> impl Stream<Item = Result<Vec<u8>, std::io::Error>> {
    codec::FramedRead::new(reader, codec::BytesCodec::new()).map_ok(|bs| bs.freeze()[..].to_vec())
}
