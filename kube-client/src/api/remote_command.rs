use std::future::Future;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Status;

use futures::{
    channel::oneshot,
    future::{
        select,
        Either::{Left, Right},
    },
    FutureExt, SinkExt, StreamExt,
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream};
use tokio_tungstenite::{tungstenite as ws, WebSocketStream};

use super::AttachParams;

type StatusReceiver = oneshot::Receiver<Status>;
type StatusSender = oneshot::Sender<Status>;

/// Errors from attaching to a pod.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to read from stdin
    #[error("failed to read from stdin: {0}")]
    ReadStdin(#[source] std::io::Error),

    /// Failed to send stdin data to the pod
    #[error("failed to send a stdin data: {0}")]
    SendStdin(#[source] ws::Error),

    /// Failed to write to stdout
    #[error("failed to write to stdout: {0}")]
    WriteStdout(#[source] std::io::Error),

    /// Failed to write to stderr
    #[error("failed to write to stderr: {0}")]
    WriteStderr(#[source] std::io::Error),

    /// Failed to receive a WebSocket message from the server.
    #[error("failed to receive a WebSocket message: {0}")]
    ReceiveWebSocketMessage(#[source] ws::Error),

    // Failed to complete the background task
    #[error("failed to complete the background task: {0}")]
    Spawn(#[source] tokio::task::JoinError),

    /// Failed to send close message.
    #[error("failed to send a WebSocket close message: {0}")]
    SendClose(#[source] ws::Error),

    /// Failed to deserialize status object
    #[error("failed to deserialize status object: {0}")]
    DeserializeStatus(#[source] serde_json::Error),

    /// Failed to send status object
    #[error("failed to send status object")]
    SendStatus,
}

const MAX_BUF_SIZE: usize = 1024;

/// Represents an attached process in a container for [`attach`] and [`exec`].
///
/// Provides access to `stdin`, `stdout`, and `stderr` if attached.
///
/// Use [`AttachedProcess::join`] to wait for the process to terminate.
///
/// [`attach`]: crate::Api::attach
/// [`exec`]: crate::Api::exec
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub struct AttachedProcess {
    has_stdin: bool,
    has_stdout: bool,
    has_stderr: bool,
    stdin_writer: Option<DuplexStream>,
    stdout_reader: Option<DuplexStream>,
    stderr_reader: Option<DuplexStream>,
    status_rx: Option<StatusReceiver>,
    task: tokio::task::JoinHandle<Result<(), Error>>,
}

impl AttachedProcess {
    pub(crate) fn new<S>(stream: WebSocketStream<S>, ap: &AttachParams) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
    {
        // To simplify the implementation, always create a pipe for stdin.
        // The caller does not have access to it unless they had requested.
        let (stdin_writer, stdin_reader) = tokio::io::duplex(ap.max_stdin_buf_size.unwrap_or(MAX_BUF_SIZE));
        let (stdout_writer, stdout_reader) = if ap.stdout {
            let (w, r) = tokio::io::duplex(ap.max_stdout_buf_size.unwrap_or(MAX_BUF_SIZE));
            (Some(w), Some(r))
        } else {
            (None, None)
        };
        let (stderr_writer, stderr_reader) = if ap.stderr {
            let (w, r) = tokio::io::duplex(ap.max_stderr_buf_size.unwrap_or(MAX_BUF_SIZE));
            (Some(w), Some(r))
        } else {
            (None, None)
        };
        let (status_tx, status_rx) = oneshot::channel();

        let task = tokio::spawn(start_message_loop(
            stream,
            stdin_reader,
            stdout_writer,
            stderr_writer,
            status_tx,
        ));

        AttachedProcess {
            has_stdin: ap.stdin,
            has_stdout: ap.stdout,
            has_stderr: ap.stderr,
            task,
            stdin_writer: Some(stdin_writer),
            stdout_reader,
            stderr_reader,
            status_rx: Some(status_rx),
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
        self.stdin_writer.take()
    }

    /// Async reader for stdout outputs.
    /// ```ignore
    /// let mut stdout_reader = attached.stdout().unwrap();
    /// let next_stdout = stdout_reader.read().await?;
    /// ```
    /// Only available if [`AttachParams`](super::AttachParams) had `stdout`.
    pub fn stdout(&mut self) -> Option<impl AsyncRead + Unpin> {
        if !self.has_stdout {
            return None;
        }
        self.stdout_reader.take()
    }

    /// Async reader for stderr outputs.
    /// ```ignore
    /// let mut stderr_reader = attached.stderr().unwrap();
    /// let next_stderr = stderr_reader.read().await?;
    /// ```
    /// Only available if [`AttachParams`](super::AttachParams) had `stderr`.
    pub fn stderr(&mut self) -> Option<impl AsyncRead + Unpin> {
        if !self.has_stderr {
            return None;
        }
        self.stderr_reader.take()
    }

    /// Abort the background task, causing remote command to fail.
    #[inline]
    pub fn abort(&self) {
        self.task.abort();
    }

    /// Waits for the remote command task to complete.
    pub async fn join(self) -> Result<(), Error> {
        self.task.await.unwrap_or_else(|e| Err(Error::Spawn(e)))
    }

    /// Take a future that resolves with any status object or when the sender is dropped.
    ///
    /// Returns `None` if called more than once.
    pub fn take_status(&mut self) -> Option<impl Future<Output = Option<Status>>> {
        self.status_rx.take().map(|recv| recv.map(|res| res.ok()))
    }
}

const STDIN_CHANNEL: u8 = 0;
const STDOUT_CHANNEL: u8 = 1;
const STDERR_CHANNEL: u8 = 2;
// status channel receives `Status` object on exit.
const STATUS_CHANNEL: u8 = 3;
// const RESIZE_CHANNEL: u8 = 4;

async fn start_message_loop<S>(
    stream: WebSocketStream<S>,
    stdin: impl AsyncRead + Unpin,
    mut stdout: Option<impl AsyncWrite + Unpin>,
    mut stderr: Option<impl AsyncWrite + Unpin>,
    status_tx: StatusSender,
) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
{
    let mut stdin_stream = tokio_util::io::ReaderStream::new(stdin);
    let (mut server_send, raw_server_recv) = stream.split();
    // Work with filtered messages to reduce noise.
    let mut server_recv = raw_server_recv.filter_map(filter_message).boxed();
    let mut server_msg = server_recv.next();
    let mut next_stdin = stdin_stream.next();

    loop {
        match select(server_msg, next_stdin).await {
            // from server
            Left((Some(message), p_next_stdin)) => {
                match message {
                    Ok(Message::Stdout(bin)) => {
                        if let Some(stdout) = stdout.as_mut() {
                            stdout.write_all(&bin[1..]).await.map_err(Error::WriteStdout)?;
                        }
                    }

                    Ok(Message::Stderr(bin)) => {
                        if let Some(stderr) = stderr.as_mut() {
                            stderr.write_all(&bin[1..]).await.map_err(Error::WriteStderr)?;
                        }
                    }

                    Ok(Message::Status(bin)) => {
                        let status =
                            serde_json::from_slice::<Status>(&bin[1..]).map_err(Error::DeserializeStatus)?;
                        status_tx.send(status).map_err(|_| Error::SendStatus)?;
                        break;
                    }

                    Err(err) => {
                        return Err(Error::ReceiveWebSocketMessage(err));
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
                        .map_err(Error::SendStdin)?;
                }
                server_msg = p_server_msg;
                next_stdin = stdin_stream.next();
            }

            Right((Some(Err(err)), _)) => {
                return Err(Error::ReadStdin(err));
            }

            Right((None, _)) => {
                // Stdin closed (writer half dropped).
                // Let the server know and disconnect.
                server_send.close().await.map_err(Error::SendClose)?;
                break;
            }
        }
    }

    Ok(())
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
