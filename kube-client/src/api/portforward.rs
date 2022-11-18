use std::{collections::HashMap, future::Future};

use bytes::{Buf, Bytes};
use futures::{
    channel::{mpsc, oneshot},
    future, FutureExt, SinkExt, StreamExt,
};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream};
use tokio_tungstenite::{tungstenite as ws, WebSocketStream};
use tokio_util::io::ReaderStream;

/// Errors from Portforwarder.
#[derive(Debug, Error)]
pub enum Error {
    /// Received invalid channel in WebSocket message.
    #[error("received invalid channel {0}")]
    InvalidChannel(usize),

    /// Received initial frame with invalid size. The initial frame must be 3 bytes, including the channel prefix.
    #[error("received initial frame with invalid size")]
    InvalidInitialFrameSize,

    /// Received initial frame with invalid port mapping.
    /// The port included in the initial frame did not match the port number associated with the channel.
    #[error("invalid port mapping in initial frame, got {actual}, expected {expected}")]
    InvalidPortMapping { actual: u16, expected: u16 },

    /// Failed to forward bytes from Pod.
    #[error("failed to forward bytes from Pod: {0}")]
    ForwardFromPod(#[source] futures::channel::mpsc::SendError),

    /// Failed to forward bytes to Pod.
    #[error("failed to forward bytes to Pod: {0}")]
    ForwardToPod(#[source] futures::channel::mpsc::SendError),

    /// Failed to write bytes from Pod.
    #[error("failed to write bytes from Pod: {0}")]
    WriteBytesFromPod(#[source] std::io::Error),

    /// Failed to read bytes to send to Pod.
    #[error("failed to read bytes to send to Pod: {0}")]
    ReadBytesToSend(#[source] std::io::Error),

    /// Received an error message from pod that is not a valid UTF-8.
    #[error("received invalid error message from Pod: {0}")]
    InvalidErrorMessage(#[source] std::string::FromUtf8Error),

    /// Failed to forward an error message from pod.
    #[error("failed to forward an error message {0:?}")]
    ForwardErrorMessage(String),

    /// Failed to send a WebSocket message to the server.
    #[error("failed to send a WebSocket message: {0}")]
    SendWebSocketMessage(#[source] ws::Error),

    /// Failed to receive a WebSocket message from the server.
    #[error("failed to receive a WebSocket message: {0}")]
    ReceiveWebSocketMessage(#[source] ws::Error),

    #[error("failed to complete the background task: {0}")]
    Spawn(#[source] tokio::task::JoinError),

    /// Failed to shutdown a pod writer channel.
    #[error("failed to shutdown write to Pod channel: {0}")]
    Shutdown(#[source] std::io::Error),
}

type ErrorReceiver = oneshot::Receiver<String>;
type ErrorSender = oneshot::Sender<String>;

// Internal message used by the futures to communicate with each other.
enum Message {
    FromPod(u8, Bytes),
    ToPod(u8, Bytes),
    FromPodClose,
    ToPodClose(u8),
}

/// Manages port-forwarded streams.
///
/// Provides `AsyncRead + AsyncWrite` for each port and **does not** bind to local ports.  Error
/// channel for each port is only written by the server when there's an exception and
/// the port cannot be used (didn't initialize or can't be used anymore).
pub struct Portforwarder {
    ports: HashMap<u16, DuplexStream>,
    errors: HashMap<u16, ErrorReceiver>,
    task: tokio::task::JoinHandle<Result<(), Error>>,
}

impl Portforwarder {
    pub(crate) fn new<S>(stream: WebSocketStream<S>, port_nums: &[u16]) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
    {
        let mut ports = HashMap::with_capacity(port_nums.len());
        let mut error_rxs = HashMap::with_capacity(port_nums.len());
        let mut error_txs = Vec::with_capacity(port_nums.len());
        let mut task_ios = Vec::with_capacity(port_nums.len());
        for port in port_nums.iter() {
            let (a, b) = tokio::io::duplex(1024 * 1024);
            ports.insert(*port, a);
            task_ios.push(b);

            let (tx, rx) = oneshot::channel();
            error_rxs.insert(*port, rx);
            error_txs.push(Some(tx));
        }
        let task = tokio::spawn(start_message_loop(
            stream,
            port_nums.to_vec(),
            task_ios,
            error_txs,
        ));

        Portforwarder {
            ports,
            errors: error_rxs,
            task,
        }
    }

    /// Take a port stream by the port on the target resource.
    ///
    /// A value is returned at most once per port.
    #[inline]
    pub fn take_stream(&mut self, port: u16) -> Option<impl AsyncRead + AsyncWrite + Unpin> {
        self.ports.remove(&port)
    }

    /// Take a future that resolves with any error message or when the error sender is dropped.
    /// When the future resolves, the port should be considered no longer usable.
    ///
    /// A value is returned at most once per port.
    #[inline]
    pub fn take_error(&mut self, port: u16) -> Option<impl Future<Output = Option<String>>> {
        self.errors.remove(&port).map(|recv| recv.map(|res| res.ok()))
    }

    /// Abort the background task, causing port forwards to fail.
    #[inline]
    pub fn abort(&self) {
        self.task.abort();
    }

    /// Waits for port forwarding task to complete.
    pub async fn join(self) -> Result<(), Error> {
        let Self {
            mut ports,
            mut errors,
            task,
        } = self;
        // Start by terminating any streams that have not yet been taken
        // since they would otherwise keep the connection open indefinitely
        ports.clear();
        errors.clear();
        task.await.unwrap_or_else(|e| Err(Error::Spawn(e)))
    }
}

async fn start_message_loop<S>(
    stream: WebSocketStream<S>,
    ports: Vec<u16>,
    duplexes: Vec<DuplexStream>,
    error_senders: Vec<Option<ErrorSender>>,
) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
{
    let mut writers = Vec::new();
    // Loops to run concurrently.
    // We can spawn tasks to run `to_pod_loop` in parallel and flatten the errors, but the other 2 loops
    // are over a single WebSocket connection and cannot process each port in parallel.
    let mut loops = Vec::with_capacity(ports.len() + 2);
    // Channel to communicate with the main loop
    let (sender, receiver) = mpsc::channel::<Message>(1);
    for (i, (r, w)) in duplexes.into_iter().map(tokio::io::split).enumerate() {
        writers.push(w);
        // Each port uses 2 channels. Duplex data channel and error.
        let ch = 2 * (i as u8);
        loops.push(to_pod_loop(ch, r, sender.clone()).boxed());
    }

    let (ws_sink, ws_stream) = stream.split();
    loops.push(from_pod_loop(ws_stream, sender).boxed());
    loops.push(forwarder_loop(&ports, receiver, ws_sink, writers, error_senders).boxed());

    future::try_join_all(loops).await.map(|_| ())
}

async fn to_pod_loop(
    ch: u8,
    reader: tokio::io::ReadHalf<DuplexStream>,
    mut sender: mpsc::Sender<Message>,
) -> Result<(), Error> {
    let mut read_stream = ReaderStream::new(reader);
    while let Some(bytes) = read_stream
        .next()
        .await
        .transpose()
        .map_err(Error::ReadBytesToSend)?
    {
        if !bytes.is_empty() {
            sender
                .send(Message::ToPod(ch, bytes))
                .await
                .map_err(Error::ForwardToPod)?;
        }
    }
    sender
        .send(Message::ToPodClose(ch))
        .await
        .map_err(Error::ForwardToPod)?;
    Ok(())
}

async fn from_pod_loop<S>(
    mut ws_stream: futures::stream::SplitStream<WebSocketStream<S>>,
    mut sender: mpsc::Sender<Message>,
) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
{
    while let Some(msg) = ws_stream
        .next()
        .await
        .transpose()
        .map_err(Error::ReceiveWebSocketMessage)?
    {
        match msg {
            ws::Message::Binary(bin) if bin.len() > 1 => {
                let mut bytes = Bytes::from(bin);
                let ch = bytes.split_to(1)[0];
                sender
                    .send(Message::FromPod(ch, bytes))
                    .await
                    .map_err(Error::ForwardFromPod)?;
            }
            message if message.is_close() => {
                sender
                    .send(Message::FromPodClose)
                    .await
                    .map_err(Error::ForwardFromPod)?;
                break;
            }
            // REVIEW should we error on unexpected websocket message?
            _ => {}
        }
    }
    Ok(())
}

// Start a loop to handle messages received from other futures.
// On `Message::ToPod(ch, bytes)`, a WebSocket message is sent with the channel prefix.
// On `Message::FromPod(ch, bytes)` with an even `ch`, `bytes` are written to the port's sink.
// On `Message::FromPod(ch, bytes)` with an odd `ch`, an error message is sent to the error channel of the port.
async fn forwarder_loop<S>(
    ports: &[u16],
    mut receiver: mpsc::Receiver<Message>,
    mut ws_sink: futures::stream::SplitSink<WebSocketStream<S>, ws::Message>,
    mut writers: Vec<tokio::io::WriteHalf<DuplexStream>>,
    mut error_senders: Vec<Option<ErrorSender>>,
) -> Result<(), Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
{
    #[derive(Default, Clone)]
    struct ChannelState {
        // Keep track if the channel has received the initialization frame.
        initialized: bool,
        // Keep track if the channel has shutdown.
        shutdown: bool,
    }
    let mut chan_state = vec![ChannelState::default(); 2 * ports.len()];
    let mut closed_ports = 0;
    let mut socket_shutdown = false;
    while let Some(msg) = receiver.next().await {
        match msg {
            Message::FromPod(ch, mut bytes) => {
                let ch = ch as usize;
                let channel = chan_state.get_mut(ch).ok_or_else(|| Error::InvalidChannel(ch))?;

                let port_index = ch / 2;
                // Initialization
                if !channel.initialized {
                    // The initial message must be 3 bytes including the channel prefix.
                    if bytes.len() != 2 {
                        return Err(Error::InvalidInitialFrameSize);
                    }

                    let port = bytes.get_u16_le();
                    if port != ports[port_index] {
                        return Err(Error::InvalidPortMapping {
                            actual: port,
                            expected: ports[port_index],
                        });
                    }

                    channel.initialized = true;
                    continue;
                }

                // Odd channels are for errors for (n - 1)/2 th port
                if ch % 2 != 0 {
                    // A port sends at most one error message because it's considered unusable after this.
                    if let Some(sender) = error_senders[port_index].take() {
                        let s = String::from_utf8(bytes.into_iter().collect())
                            .map_err(Error::InvalidErrorMessage)?;
                        sender.send(s).map_err(Error::ForwardErrorMessage)?;
                    }
                } else if !channel.shutdown {
                    writers[port_index]
                        .write_all(&bytes)
                        .await
                        .map_err(Error::WriteBytesFromPod)?;
                }
            }

            Message::ToPod(ch, bytes) => {
                let mut bin = Vec::with_capacity(bytes.len() + 1);
                bin.push(ch);
                bin.extend(bytes.into_iter());
                ws_sink
                    .send(ws::Message::binary(bin))
                    .await
                    .map_err(Error::SendWebSocketMessage)?;
            }
            Message::ToPodClose(ch) => {
                let ch = ch as usize;
                let channel = chan_state.get_mut(ch).ok_or_else(|| Error::InvalidChannel(ch))?;
                let port_index = ch / 2;

                if !channel.shutdown {
                    writers[port_index].shutdown().await.map_err(Error::Shutdown)?;
                    channel.shutdown = true;

                    closed_ports += 1;
                }
            }
            Message::FromPodClose => {
                for writer in &mut writers {
                    writer.shutdown().await.map_err(Error::Shutdown)?;
                }
            }
        }

        if closed_ports == ports.len() && !socket_shutdown {
            ws_sink
                .send(ws::Message::Close(None))
                .await
                .map_err(Error::SendWebSocketMessage)?;
            socket_shutdown = true;
        }
    }
    Ok(())
}
