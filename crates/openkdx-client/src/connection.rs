//! One KDX connection: a reader task that routes replies to their requests
//! and everything else to an event stream, and a writer behind a channel.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::{SinkExt, StreamExt};
use openkdx_codec::{Event as CodecEvent, Item, SessionCodec};
use openkdx_proto::msg::admin::DisconnectMessage;
use openkdx_proto::msg::error_name;
use openkdx_proto::session::Header;
use openkdx_proto::wire::DecodeError;
use openkdx_proto::{AccountKey, Message, MessageType, Payload};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::AbortHandle;
use tokio_util::codec::{Framed, FramedParts};

use crate::txid::TxidSource;
use crate::{ClientError, Result};

/// Why a connection ended.
#[derive(Debug, Clone, Default)]
pub struct Disconnect {
    /// The 151 Disconnect Message, if the server sent one first.
    pub message: Option<DisconnectMessage>,
    /// A framing or protocol fault on our side of the read, if any; `None`
    /// for a plain close.
    pub error: Option<Arc<ClientError>>,
}

/// What arrives without being the reply to one of our requests.
#[derive(Debug, Clone)]
pub enum Event {
    Message(Box<Message>),
    /// A frame of a known type whose payload did not decode.
    Undecodable {
        kind: MessageType,
        error: DecodeError,
        bytes: Bytes,
    },
    Disconnected(Disconnect),
}

type Reply = (Message, Bytes);
type PendingMap = HashMap<u32, oneshot::Sender<Reply>>;

enum Outgoing {
    Item(Item, oneshot::Sender<Result<()>>),
    Close(oneshot::Sender<()>),
}

/// State the reader and the handles share.
struct Shared {
    pending: Mutex<Option<PendingMap>>,
    last_151: Mutex<Option<DisconnectMessage>>,
    events: broadcast::Sender<Event>,
    closed: AtomicBool,
    finalized: AtomicBool,
}

impl Shared {
    /// Mark the connection dead once: fail every waiter, tell subscribers.
    fn finalize(&self, error: Option<ClientError>) {
        if self.finalized.swap(true, Ordering::SeqCst) {
            return;
        }
        self.closed.store(true, Ordering::SeqCst);
        // Dropping the senders makes every pending request see Disconnected.
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let message = self
            .last_151
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let _ = self.events.send(Event::Disconnected(Disconnect {
            message,
            error: error.map(Arc::new),
        }));
    }
}

#[derive(Clone)]
pub struct Connection {
    writer: mpsc::Sender<Outgoing>,
    shared: Arc<Shared>,
    reader: Arc<Mutex<Option<AbortHandle>>>,
    txids: Arc<TxidSource>,
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
    /// the handshake read past the proof; `max_len` is the receive cap. The
    /// returned receiver was subscribed before the reader started, so it
    /// misses nothing.
    pub fn spawn(
        stream: TcpStream,
        read_buf: BytesMut,
        key: &AccountKey,
        max_len: u64,
    ) -> (Self, broadcast::Receiver<Event>) {
        let local_addr = stream.local_addr().ok();
        let peer_addr = stream.peer_addr().ok();
        let mut parts = FramedParts::new::<Item>(stream, SessionCodec::new(key, max_len));
        parts.read_buf = read_buf;
        let (mut sink, mut source) = Framed::from_parts(parts).split();

        let (writer, mut outgoing) = mpsc::channel::<Outgoing>(64);
        let (events, first_receiver) = broadcast::channel(256);
        let shared = Arc::new(Shared {
            pending: Mutex::new(Some(HashMap::new())),
            last_151: Mutex::new(None),
            events,
            closed: AtomicBool::new(false),
            finalized: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            while let Some(out) = outgoing.recv().await {
                match out {
                    Outgoing::Item(item, ack) => {
                        let result = sink.send(item).await.map_err(ClientError::from);
                        let failed = result.is_err();
                        let _ = ack.send(result);
                        if failed {
                            break;
                        }
                    }
                    Outgoing::Close(ack) => {
                        let _ = sink.close().await;
                        let _ = ack.send(());
                        break;
                    }
                }
            }
        });

        let reader_shared = shared.clone();
        let reader_writer = writer.clone();
        let reader = tokio::spawn(async move {
            let error = loop {
                match source.next().await {
                    Some(Ok(CodecEvent::Message { header, payload })) => {
                        reader_shared.deliver(header, payload);
                    }
                    Some(Ok(_)) => {
                        break Some(ClientError::Protocol("data on a message connection"));
                    }
                    Some(Err(e)) => break Some(ClientError::Codec(e)),
                    None => break None,
                }
            };
            reader_shared.finalize(error);
            let (ack, _) = oneshot::channel();
            let _ = reader_writer.try_send(Outgoing::Close(ack));
        });

        let conn = Connection {
            writer,
            shared,
            reader: Arc::new(Mutex::new(Some(reader.abort_handle()))),
            txids: Arc::new(TxidSource::new_random()),
            timeout: Duration::from_secs(30),
            local_addr,
            peer_addr,
        };
        (conn, first_receiver)
    }

    /// The request timeout for this handle (clones keep their own).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.shared.events.subscribe()
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    /// The 151 Disconnect Message, once one has arrived.
    pub fn disconnect_message(&self) -> Option<DisconnectMessage> {
        self.shared
            .last_151
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Send a message that expects no reply; returns its txid once the
    /// bytes have been written.
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
            let mut guard = self
                .shared
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let Some(pending) = guard.as_mut() else {
                return Err(self.disconnected());
            };
            pending.insert(txid, tx);
        }
        if let Err(e) = self.send_with_txid(txid, req).await {
            self.forget(txid);
            return Err(e);
        }
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) => Err(self.disconnected()),
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

    /// Write one codec item and wait until it has gone out (transfers use
    /// this for data headers and data).
    pub(crate) async fn send_item(&self, item: Item) -> Result<()> {
        if self.is_closed() {
            return Err(self.disconnected());
        }
        let (ack, done) = oneshot::channel();
        self.writer
            .send(Outgoing::Item(item, ack))
            .await
            .map_err(|_| self.disconnected())?;
        done.await.map_err(|_| self.disconnected())?
    }

    fn disconnected(&self) -> ClientError {
        match self.disconnect_message() {
            Some(m) => ClientError::DisconnectedBy(Box::new(m)),
            None => ClientError::Disconnected,
        }
    }

    fn forget(&self, txid: u32) {
        if let Some(p) = self
            .shared
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            p.remove(&txid);
        }
    }

    /// Shut the socket down for writing, give the peer a moment to close,
    /// then end the reader. Afterwards `is_closed()` is true, every pending
    /// request has failed, and subscribers have seen `Disconnected`.
    pub async fn close(&self) {
        let (ack, done) = oneshot::channel();
        if self.writer.send(Outgoing::Close(ack)).await.is_ok() {
            let _ = tokio::time::timeout(Duration::from_secs(2), done).await;
        }
        let mut events = self.events();
        let graceful = tokio::time::timeout(Duration::from_secs(1), async {
            while !self.is_closed() {
                if events.recv().await.is_err() {
                    break;
                }
            }
        })
        .await;
        if graceful.is_err()
            && let Some(h) = self
                .reader
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take()
        {
            h.abort();
        }
        self.shared.finalize(None);
    }
}

impl Shared {
    fn deliver(&self, header: Header, payload: Bytes) {
        let kind = MessageType(header.kind);
        let message = match Message::decode(kind, &payload) {
            Ok(m) => m,
            Err(error) => {
                if !header.reply
                    || !self.take_waiter(
                        header.txid,
                        Message::Unknown {
                            kind,
                            bytes: payload.to_vec(),
                        },
                        payload.clone(),
                    )
                {
                    let _ = self.events.send(Event::Undecodable {
                        kind,
                        error,
                        bytes: payload,
                    });
                }
                return;
            }
        };
        if header.reply {
            let message = match self.try_take_waiter(header.txid, message, payload) {
                Ok(()) => return,
                Err(m) => *m,
            };
            self.publish(message);
            return;
        }
        self.publish(message);
    }

    fn publish(&self, message: Message) {
        if let Message::DisconnectMessage(d) = &message {
            *self.last_151.lock().unwrap_or_else(PoisonError::into_inner) = Some(d.clone());
        }
        let _ = self.events.send(Event::Message(Box::new(message)));
    }

    /// Hand a reply to its waiter; `Err` gives the message back when nobody
    /// is waiting for that txid.
    fn try_take_waiter(
        &self,
        txid: u32,
        message: Message,
        payload: Bytes,
    ) -> std::result::Result<(), Box<Message>> {
        let waiter = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .and_then(|p| p.remove(&txid));
        match waiter {
            Some(tx) => {
                let _ = tx.send((message, payload));
                Ok(())
            }
            None => Err(Box::new(message)),
        }
    }

    fn take_waiter(&self, txid: u32, message: Message, payload: Bytes) -> bool {
        self.try_take_waiter(txid, message, payload).is_ok()
    }
}
