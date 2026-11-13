//! An in-process KDX server for tests: real handshake and framing from
//! `openkdx-proto`/`openkdx-codec`, with a scripted session behind it.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use openkdx_codec::{Event, HandshakeCodec, HandshakeFrame, Item, SessionCodec};
use openkdx_proto::handshake::{Hello, Proof, ProofStatus, SubProtocol, proof_auth};
use openkdx_proto::session::{Header, SERVER_RECV_CAP};
use openkdx_proto::{AccountKey, ChallengeTable, Message, MessageType};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_util::codec::{Framed, FramedParts};

#[derive(Debug, Clone)]
pub enum Step {
    /// Read one message and require this type; remember its txid and payload.
    Expect(MessageType),
    /// Reply to the last expected message (its txid, reply flag set).
    Reply(Message),
    /// Send with the server's own txid.
    Send(Message),
    /// A 414-style data header (reply to the last expected message when
    /// `reply`), then the bytes.
    DataReply {
        kind: MessageType,
        reply: bool,
        bytes: Vec<u8>,
    },
    /// Consume one data header and its bytes; record them.
    ExpectData,
    Close,
    Sleep(Duration),
}

pub type Script = Vec<Step>;

/// What the client sent: `(kind, txid, payload)` for messages, and the bytes
/// of each data transfer.
#[derive(Debug, Default)]
pub struct Captured {
    pub messages: Vec<(MessageType, u32, Vec<u8>)>,
    pub data: Vec<(u64, Vec<u8>)>,
    /// `(txid, reply flag)` of each data header the client sent.
    pub data_headers: Vec<(u32, bool)>,
    /// Script expectations the client violated.
    pub failures: Vec<String>,
}

/// How the mock presents itself and how it rejects a bad login.
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// 0x20 server, 0x30 tracker.
    pub sub_version: u8,
    /// `None`: close the connection on a bad proof, as Server 1.620 does.
    /// `Some(status)`: answer with a failure proof carrying that status.
    pub reject: Option<ProofStatus>,
}

impl Default for MockConfig {
    fn default() -> Self {
        MockConfig {
            sub_version: 0x20,
            reject: None,
        }
    }
}

pub struct MockServer {
    pub addr: SocketAddr,
    pub captured: Arc<Mutex<Captured>>,
    handle: JoinHandle<()>,
}

pub fn table() -> ChallengeTable {
    ChallengeTable::default_table()
}

pub fn key(password: &str) -> AccountKey {
    AccountKey::from_password(password.as_bytes())
}

impl MockServer {
    /// Accept one connection per script, in order. `accounts` maps login names
    /// to account keys; an unknown login or bad proof is rejected.
    pub async fn start(accounts: Vec<(Vec<u8>, AccountKey)>, scripts: Vec<Script>) -> MockServer {
        Self::start_as(0x20, accounts, scripts).await
    }

    pub async fn start_as(
        sub_version: u8,
        accounts: Vec<(Vec<u8>, AccountKey)>,
        scripts: Vec<Script>,
    ) -> MockServer {
        Self::start_with(
            MockConfig {
                sub_version,
                ..MockConfig::default()
            },
            accounts,
            scripts,
        )
        .await
    }

    pub async fn start_with(
        config: MockConfig,
        accounts: Vec<(Vec<u8>, AccountKey)>,
        scripts: Vec<Script>,
    ) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Captured::default()));
        let cap = captured.clone();
        let handle = tokio::spawn(async move {
            let mut tasks = Vec::new();
            for script in scripts {
                let (stream, _) = listener.accept().await.unwrap();
                let accounts = accounts.clone();
                let cap = cap.clone();
                let config = config.clone();
                tasks.push(tokio::spawn(async move {
                    serve(stream, config, accounts, script, cap).await;
                }));
            }
            for t in tasks {
                let _ = t.await;
            }
        });
        MockServer {
            addr,
            captured,
            handle,
        }
    }

    pub async fn finish(self) {
        tokio::time::timeout(Duration::from_secs(5), self.handle)
            .await
            .expect("mock server finished")
            .unwrap();
    }

    /// Panic in the test if the client violated the script.
    pub fn check(&self) {
        let failures = &self.captured.lock().unwrap().failures;
        assert!(failures.is_empty(), "mock script violated: {failures:?}");
    }

    pub fn payloads(&self, kind: MessageType) -> Vec<Vec<u8>> {
        self.captured
            .lock()
            .unwrap()
            .messages
            .iter()
            .filter(|(k, _, _)| *k == kind)
            .map(|(_, _, p)| p.clone())
            .collect()
    }
}

async fn serve(
    stream: TcpStream,
    config: MockConfig,
    accounts: Vec<(Vec<u8>, AccountKey)>,
    script: Script,
    captured: Arc<Mutex<Captured>>,
) {
    let mut framed = Framed::new(stream, HandshakeCodec::new(0x1234_5678));
    let Some(Ok(HandshakeFrame::Hello(client_hello))) = framed.next().await else {
        return;
    };
    let nonce = [7u8; 12];
    let our_hello_challenge = 3u16;
    let our_hello = Hello {
        sub_protocol: SubProtocol::Kdx,
        version: 1,
        sub_version: config.sub_version,
        challenge: our_hello_challenge,
        nonce,
        name: Vec::new(),
        pad_len: 0,
        skipped_len: 0,
        pad: Vec::new(),
        skipped: Vec::new(),
        ignored: [0; 4],
        padding: Vec::new(),
    };
    framed.send(HandshakeFrame::Hello(our_hello)).await.unwrap();
    let Some(Ok(HandshakeFrame::Proof(client_proof))) = framed.next().await else {
        return;
    };
    let key = accounts
        .iter()
        .find(|(login, _)| *login == client_hello.name)
        .map(|(_, k)| *k);
    let ok = key.is_some_and(|k| proof_auth(&k, &nonce) == client_proof.auth)
        && table().verify(our_hello_challenge, &client_proof.response);
    let Some(key) = key.filter(|_| ok) else {
        if let Some(status) = config.reject {
            let _ = framed
                .send(HandshakeFrame::Proof(Proof::failure(
                    &client_hello.nonce,
                    status,
                )))
                .await;
        }
        return;
    };
    let proof =
        Proof::compute(&key, &client_hello.nonce, &table(), client_hello.challenge).unwrap();
    framed.send(HandshakeFrame::Proof(proof)).await.unwrap();

    let parts = framed.into_parts();
    let mut fp = FramedParts::new::<Item>(parts.io, SessionCodec::new(&key, SERVER_RECV_CAP));
    fp.read_buf = parts.read_buf;
    let mut session = Framed::from_parts(fp);

    let mut last_txid = 0u32;
    let mut own_txid = 0xc3b7_1d00u32;
    for step in script {
        match step {
            Step::Expect(kind) => match session.next().await {
                Some(Ok(Event::Message { header, payload })) => {
                    let got = MessageType(header.kind);
                    let mut cap = captured.lock().unwrap();
                    cap.messages.push((got, header.txid, payload.to_vec()));
                    if got != kind {
                        cap.failures.push(format!("expected {kind}, got {got}"));
                        return;
                    }
                    last_txid = header.txid;
                }
                other => {
                    captured
                        .lock()
                        .unwrap()
                        .failures
                        .push(format!("expected {kind}, got {other:?}"));
                    return;
                }
            },
            Step::Reply(msg) => {
                let payload = msg.encode();
                let header = Header::reply(last_txid, msg.kind().0, payload.len() as u64);
                session
                    .send(Item::Message {
                        header,
                        payload: Bytes::from(payload),
                    })
                    .await
                    .unwrap();
            }
            Step::Send(msg) => {
                own_txid += 1;
                let payload = msg.encode();
                let header = Header::request(own_txid, msg.kind().0, payload.len() as u64);
                session
                    .send(Item::Message {
                        header,
                        payload: Bytes::from(payload),
                    })
                    .await
                    .unwrap();
            }
            Step::DataReply { kind, reply, bytes } => {
                let header = if reply {
                    Header::reply(last_txid, kind.0, bytes.len() as u64)
                } else {
                    own_txid += 1;
                    Header::request(own_txid, kind.0, bytes.len() as u64)
                };
                session.send(Item::DataHeader(header)).await.unwrap();
                if !bytes.is_empty() {
                    session.send(Item::Data(Bytes::from(bytes))).await.unwrap();
                }
            }
            Step::ExpectData => {
                let mut got = Vec::new();
                let mut declared = 0;
                loop {
                    match session.next().await {
                        Some(Ok(Event::DataStart(h))) => {
                            declared = h.len;
                            captured
                                .lock()
                                .unwrap()
                                .data_headers
                                .push((h.txid, h.reply));
                        }
                        Some(Ok(Event::Data(b))) => got.extend_from_slice(&b),
                        Some(Ok(Event::DataEnd)) => break,
                        other => {
                            captured
                                .lock()
                                .unwrap()
                                .failures
                                .push(format!("expected data, got {other:?}"));
                            return;
                        }
                    }
                }
                captured.lock().unwrap().data.push((declared, got));
            }
            Step::Close => return,
            Step::Sleep(d) => tokio::time::sleep(d).await,
        }
    }
    // Keep the connection open until the client goes away.
    while let Some(Ok(_)) = session.next().await {}
}
