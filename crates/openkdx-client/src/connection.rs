//! One KDX connection: a reader task that routes replies to their requests
//! and everything else to an event stream, and a writer behind a channel.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use openkdx_codec::{Event as CodecEvent, Item, SessionCodec};
use openkdx_proto::msg::admin::DisconnectMessage;
use openkdx_proto::msg::error_name;
use openkdx_proto::session::Header;
use openkdx_proto::{AccountKey, Message, MessageType, Payload};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::codec::{Framed, FramedParts};

use crate::txid::TxidSource;
use crate::{ClientError, Result};

/// What arrives without being the reply to one of our requests.
#[derive(Debug, Clone)]
pub enum Event {
    Message(Box<Message>),
    /// The connection ended; with the 151 Disconnect Message if one came.
    Disconnected(Option<DisconnectMessage>),
}

type Reply = (Message, Bytes);
type Pending = Arc<Mutex<Option<HashMap<u32, oneshot::Sender<Reply>>>>>;

enum Outgoing {
    Item(Item),
    Close,
}

#[derive(Clone)]
pub struct Connection {
    writer: mpsc::Sender<Outgoing>,
    pending: Pending,
    txids: Arc<TxidSource>,
    events: broadcast::Sender<Event>,
    closed: Arc<AtomicBool>,
    timeout: Duration,
    local_addr: Option<SocketAddr>,
    peer_addr: Option<SocketAddr>,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("peer_addr", &self.peer_addr)
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl Connection {
    /// Take over a stream whose handshake is done. `read_buf` holds whatever
    /// the handshake read past the proof; `max_len` is the receive cap.
    pub fn spawn(stream: TcpStream, read_buf: BytesMut, key: &AccountKey, max_len: u64) -> Self {
        let local_addr = stream.local_addr().ok();
        let peer_addr = stream.peer_addr().ok();
        let mut parts = FramedParts::new::<Item>(stream, SessionCodec::new(key, max_len));
        parts.read_buf = read_buf;
        let framed = Framed::from_parts(parts);
        let (mut sink, mut source) = framed.split();

        let (writer, mut outgoing) = mpsc::channel::<Outgoing>(64);
        let (events, _) = broadcast::channel(256);
        let pending: Pending = Arc::new(Mutex::new(Some(HashMap::new())));
        let closed = Arc::new(AtomicBool::new(false));

        tokio::spawn(async move {
            while let Some(Outgoing::Item(item)) = outgoing.recv().await {
                if sink.send(item).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        let reader_pending = pending.clone();
        let reader_events = events.clone();
        let reader_closed = closed.clone();
        tokio::spawn(async move {
            let mut last_151 = None;
            loop {
                let frame = source.next().await;
                let Some(Ok(CodecEvent::Message { header, payload })) = frame else {
                    break;
                };
                let kind = MessageType(header.kind);
                let message = match Message::decode(kind, &payload) {
                    Ok(m) => m,
                    Err(_) => Message::Unknown {
                        kind,
                        bytes: payload.to_vec(),
                    },
                };
                if header.reply {
                    let waiter = reader_pending
                        .lock()
                        .unwrap()
                        .as_mut()
                        .and_then(|p| p.remove(&header.txid));
                    if let Some(tx) = waiter {
                        let _ = tx.send((message, payload));
                        continue;
                    }
                }
                if let Message::DisconnectMessage(d) = &message {
                    last_151 = Some(d.clone());
                }
                let _ = reader_events.send(Event::Message(Box::new(message)));
            }
            reader_closed.store(true, Ordering::SeqCst);
            // Drop every pending sender: their receivers see Disconnected.
            reader_pending.lock().unwrap().take();
            let _ = reader_events.send(Event::Disconnected(last_151));
        });

        Connection {
            writer,
            pending,
            txids: Arc::new(TxidSource::new_random()),
            events,
            closed,
            timeout: Duration::from_secs(30),
            local_addr,
            peer_addr,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    /// Send a message that expects no reply; returns its txid.
    pub async fn send<T: Payload>(&self, msg: &T) -> Result<u32> {
        let txid = self.txids.next();
        self.send_with_txid(txid, msg).await?;
        Ok(txid)
    }

    async fn send_with_txid<T: Payload>(&self, txid: u32, msg: &T) -> Result<()> {
        let payload = msg.to_bytes();
        let header = Header::request(txid, T::KIND.0, payload.len() as u64);
        self.send_item(Item::Message {
            header,
            payload: Bytes::from(payload),
        })
        .await
    }

    /// Send a request and wait for the frame that answers it: the reply flag
    /// and our txid. The raw message, which may be a `Status`.
    pub async fn request<T: Payload>(&self, req: &T) -> Result<Message> {
        Ok(self.request_raw(req).await?.0)
    }

    async fn request_raw<T: Payload>(&self, req: &T) -> Result<Reply> {
        let txid = self.txids.next();
        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.pending.lock().unwrap();
            let Some(pending) = guard.as_mut() else {
                return Err(ClientError::Disconnected);
            };
            pending.insert(txid, tx);
        }
        if let Err(e) = self.send_with_txid(txid, req).await {
            self.forget(txid);
            return Err(e);
        }
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(message)) => Ok(message),
            Ok(Err(_)) => Err(ClientError::Disconnected),
            Err(_) => {
                self.forget(txid);
                Err(ClientError::Timeout)
            }
        }
    }

    /// `request`, then insist on type `R`: a `Status` error becomes
    /// `ClientError::Server`, anything else `Unexpected`.
    pub async fn request_as<T: Payload, R: Payload>(&self, req: &T) -> Result<R> {
        let (message, payload) = self.request_raw(req).await?;
        let kind = message.kind();
        if kind == R::KIND {
            return R::decode(&payload).map_err(Into::into);
        }
        if let Message::Status(status) = &message
            && status.is_error()
        {
            return Err(ClientError::Server {
                code: status.code,
                name: error_name(status.code),
                request: T::KIND.0,
            });
        }
        Err(ClientError::Unexpected {
            expected: R::KIND,
            got: kind,
        })
    }

    /// Hand the raw writer an item (used by transfers).
    pub(crate) async fn send_item(&self, item: Item) -> Result<()> {
        self.writer
            .send(Outgoing::Item(item))
            .await
            .map_err(|_| ClientError::Disconnected)
    }

    fn forget(&self, txid: u32) {
        if let Some(p) = self.pending.lock().unwrap().as_mut() {
            p.remove(&txid);
        }
    }

    /// Shut the socket down for writing; the reader then sees the peer close.
    pub async fn close(&self) {
        let _ = self.writer.send(Outgoing::Close).await;
    }
}
