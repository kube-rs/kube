use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use futures::{
    channel::{mpsc, oneshot},
    FutureExt, SinkExt, StreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, DuplexStream};
use tokio_tungstenite::{tungstenite as ws, WebSocketStream};

type ErrorReceiver = oneshot::Receiver<String>;
type ErrorSender = oneshot::Sender<String>;

enum Message {
    FromPod(Vec<u8>),
    ToPod(Vec<u8>),
}

struct PortforwarderState {
    waker: Option<Waker>,
    finished: bool,
    ports: Option<Vec<Port>>,
}

// Provides `AsyncRead + AsyncWrite` for each port and **does not** bind to local ports.
// Error channel for each port is only written by the server when there's an exception and
// the port cannot be used (didn't initialize or can't be used anymore).
/// Manage port forwarding.
pub struct Portforwarder {
    state: Arc<Mutex<PortforwarderState>>,
}

impl Portforwarder {
    pub(crate) fn new<S>(stream: WebSocketStream<S>, port_nums: &[u16]) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
    {
        let mut ports = Vec::new();
        let mut errors = Vec::new();
        let mut duplexes = Vec::new();
        for _ in port_nums.iter() {
            let (a, b) = tokio::io::duplex(1024 * 1024);
            let (tx, rx) = oneshot::channel();
            ports.push(Port::new(a, rx));
            errors.push(Some(tx));
            duplexes.push(b);
        }

        let state = Arc::new(Mutex::new(PortforwarderState {
            waker: None,
            finished: false,
            ports: Some(ports),
        }));
        let shared_state = state.clone();
        let port_nums = port_nums.to_owned();
        tokio::spawn(async move {
            start_message_loop(stream, &port_nums, duplexes, errors).await;

            let mut shared = shared_state.lock().unwrap();
            shared.finished = true;
            if let Some(waker) = shared.waker.take() {
                waker.wake()
            }
        });
        Portforwarder { state }
    }

    /// Get streams for forwarded ports.
    pub fn ports(&mut self) -> Option<Vec<Port>> {
        let mut state = self.state.lock().unwrap();
        state.ports.take()
    }
}

impl Future for Portforwarder {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if state.finished {
            return Poll::Ready(());
        }

        if let Some(waker) = &state.waker {
            if waker.will_wake(cx.waker()) {
                return Poll::Pending;
            }
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub struct Port {
    // Data pipe.
    stream: Option<DuplexStream>,
    // Error channel.
    error: Option<ErrorReceiver>,
}

impl Port {
    pub(crate) fn new(stream: DuplexStream, error: ErrorReceiver) -> Self {
        Port {
            stream: Some(stream),
            error: Some(error),
        }
    }

    /// Data pipe for sending to and receiving from the forwarded port.
    pub fn stream(&mut self) -> Option<impl AsyncRead + AsyncWrite + Unpin> {
        self.stream.take()
    }

    /// Future that resolves with any error message or when the error sender is dropped.
    /// When this resolves, the port should be considered no longer usable.
    pub fn error(&mut self) -> Option<impl Future<Output = Option<String>>> {
        // Ignore Cancellation error.
        self.error.take().map(|recv| recv.map(|res| res.ok()))
    }
}

async fn start_message_loop<S>(
    stream: WebSocketStream<S>,
    ports: &[u16],
    duplexes: Vec<DuplexStream>,
    mut errors: Vec<Option<ErrorSender>>,
) where
    S: AsyncRead + AsyncWrite + Unpin + Sized + Send + 'static,
{
    let mut writers = Vec::new();
    let (tx, mut rx) = mpsc::channel::<Message>(1);
    for (i, (r, w)) in duplexes.into_iter().map(tokio::io::split).enumerate() {
        writers.push(w);
        {
            // Each port uses 2 channels. Duplex data channel and error.
            let ch = 2 * (i as u8);
            let mut read_stream = tokio_util::io::ReaderStream::new(r);
            let mut txr = tx.clone();
            tokio::spawn(async move {
                while let Some(res) = read_stream.next().await {
                    match res {
                        Ok(bytes) => {
                            if !bytes.is_empty() {
                                // Prefix the message with its channel byte.
                                let mut vec = Vec::with_capacity(bytes.len() + 1);
                                vec.push(ch);
                                vec.extend_from_slice(&bytes[..]);
                                // TODO Maybe use one shot to check for error and break
                                txr.send(Message::ToPod(vec)).await.expect("send message")
                            }
                        }
                        Err(err) => panic!("{}", err),
                    }
                }
            });
        }
    }

    let (mut server_send, mut server_recv) = stream.split();
    {
        let mut txs = tx.clone();
        tokio::spawn(async move {
            while let Some(res) = server_recv.next().await {
                match res {
                    Ok(ws::Message::Binary(bin)) if bin.len() > 1 => {
                        txs.send(Message::FromPod(bin)).await.unwrap()
                    }
                    Ok(_) => {}
                    Err(err) => panic!("{}", err),
                }
            }
        });
    }
    // Drop the original so the stream terminates.
    drop(tx);

    // Keep track if the channel has received initialization frame.
    let mut initialized = vec![false; 2 * ports.len()];
    while let Some(msg) = rx.next().await {
        match msg {
            Message::FromPod(bin) => {
                let ch = bin[0] as usize;
                if ch >= initialized.len() {
                    panic!("Unexpected channel {}", ch);
                }

                // Odd channels are for errors for (n - 1)/2 th port
                let is_error = ch % 2 == 1;
                let port_index = ch / 2;
                if !initialized[ch] {
                    if bin.len() != 3 {
                        panic!("Unexpected initial channel frame size");
                    }

                    let port = u16::from_le_bytes([bin[1], bin[2]]);
                    if port != ports[port_index] {
                        panic!(
                            "Unexpected port number in initial frame ({} != {})",
                            port, ports[port_index]
                        );
                    }

                    initialized[ch] = true;
                } else if !is_error {
                    writers[port_index].write_all(&bin[1..]).await.expect("writable");
                } else if let Some(sender) = errors[port_index].take() {
                    let s = String::from_utf8(bin[1..].to_vec()).expect("valid error message");
                    sender.send(s).expect("send error");
                }
            }

            Message::ToPod(bin) => {
                server_send
                    .send(ws::Message::binary(bin))
                    .await
                    .expect("send to pod");
            }
        }
    }
}
