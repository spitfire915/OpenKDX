//! Decoding one KDX connection: the handshake, the key, and the session.

use std::fmt;
use std::net::SocketAddr;
use std::path::Path;

use openkdx_proto::account::AccountKey;
use openkdx_proto::challenge::ChallengeTable;
use openkdx_proto::handshake::{HandshakeError, Hello, Proof, Role, proof_auth};
use openkdx_proto::message_type::MessageType;
use openkdx_proto::msg::Message;
use openkdx_proto::registration::{Registration, RegistrationError};
use openkdx_proto::session::{
    CLIENT_RECV_CAP, Header, SERVER_RECV_CAP, SessionCipher, TRACKER_RECV_CAP,
};
use openkdx_proto::wire::DecodeError;

use crate::capture::{self, Flow, Stream};

/// The ports that tell a connection's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ports {
    pub dcc: u16,
    pub server: u16,
    pub tracker: u16,
    pub gconsole: u16,
}

impl Default for Ports {
    fn default() -> Self {
        Ports {
            dcc: 10600,
            server: 10700,
            tracker: 10800,
            gconsole: 10900,
        }
    }
}

impl Ports {
    pub fn kind_of(&self, port: u16) -> Option<ConnectionKind> {
        if port == self.server {
            Some(ConnectionKind::Server)
        } else if port == self.tracker {
            Some(ConnectionKind::Tracker)
        } else if port == self.dcc {
            Some(ConnectionKind::Dcc)
        } else if port == self.gconsole {
            Some(ConnectionKind::GConsole)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Dcc,
    Server,
    Tracker,
    GConsole,
}

impl ConnectionKind {
    /// What the client accepts from this kind of peer.
    pub fn client_recv_cap(self) -> u64 {
        match self {
            ConnectionKind::Tracker => TRACKER_RECV_CAP,
            _ => CLIENT_RECV_CAP,
        }
    }

    /// What this kind of peer accepts from the client.
    pub fn server_recv_cap(self) -> u64 {
        match self {
            ConnectionKind::Tracker => TRACKER_RECV_CAP,
            _ => SERVER_RECV_CAP,
        }
    }
}

impl fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ConnectionKind::Dcc => "DCC",
            ConnectionKind::Server => "server",
            ConnectionKind::Tracker => "tracker",
            ConnectionKind::GConsole => "GConsole",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    ClientToServer,
    ServerToClient,
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Dir::ClientToServer => "C->S",
            Dir::ServerToClient => "S->C",
        })
    }
}

/// Where a connection's key came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySource {
    /// `--password #1`, `empty password`, or `105 Server Info of <client>`.
    pub label: String,
    pub key: AccountKey,
}

/// One decoded unit of a connection, in stream order.
#[derive(Debug, Clone)]
pub enum Record {
    Hello {
        dir: Dir,
        frame: usize,
        hello: Hello,
    },
    Proof {
        dir: Dir,
        frame: usize,
        proof: Proof,
        /// `None` until the key is known.
        auth_ok: Option<bool>,
        /// `None` without a challenge table.
        challenge_ok: Option<bool>,
    },
    Message {
        dir: Dir,
        frame: usize,
        header: Header,
        payload: Vec<u8>,
        message: Result<Message, DecodeError>,
    },
    /// A 414 or 417 header: the bytes that follow are transfer data.
    DataHeader {
        dir: Dir,
        frame: usize,
        header: Header,
    },
    Data {
        dir: Dir,
        frame: usize,
        len: u64,
    },
    Stopped {
        dir: Dir,
        frame: usize,
        reason: String,
    },
}

impl Record {
    pub fn dir(&self) -> Dir {
        match self {
            Record::Hello { dir, .. }
            | Record::Proof { dir, .. }
            | Record::Message { dir, .. }
            | Record::DataHeader { dir, .. }
            | Record::Data { dir, .. }
            | Record::Stopped { dir, .. } => *dir,
        }
    }

    pub fn frame(&self) -> usize {
        match self {
            Record::Hello { frame, .. }
            | Record::Proof { frame, .. }
            | Record::Message { frame, .. }
            | Record::DataHeader { frame, .. }
            | Record::Data { frame, .. }
            | Record::Stopped { frame, .. } => *frame,
        }
    }

    /// The message type of a message or data header.
    pub fn kind(&self) -> Option<u32> {
        match self {
            Record::Message { header, .. } | Record::DataHeader { header, .. } => Some(header.kind),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub flow_index: usize,
    pub kind: ConnectionKind,
    pub client: SocketAddr,
    pub server: SocketAddr,
    pub key: Option<KeySource>,
    pub records: Vec<Record>,
    /// Keys this connection revealed for later connections (105 Server Info).
    pub harvested_keys: Vec<KeySource>,
}

#[derive(Debug, Clone)]
pub struct RegistrationRecord {
    pub frame: usize,
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub datagram: Vec<u8>,
    pub registration: Result<Registration, RegistrationError>,
}

#[derive(Debug, Clone)]
pub struct Capture {
    pub connections: Vec<Connection>,
    pub registrations: Vec<RegistrationRecord>,
    pub table_present: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub ports: Ports,
    /// Candidate passwords; the empty password is always tried last.
    pub passwords: Vec<Vec<u8>>,
    pub table: Option<ChallengeTable>,
}

impl Options {
    fn candidates(&self) -> Vec<KeySource> {
        let mut out: Vec<KeySource> = self
            .passwords
            .iter()
            .enumerate()
            .map(|(i, p)| KeySource {
                label: format!("--password #{}", i + 1),
                key: AccountKey::from_password(p),
            })
            .collect();
        out.push(KeySource {
            label: "empty password".into(),
            key: AccountKey::GUEST,
        });
        out
    }
}

/// Reads a capture and decodes every KDX connection and registration in it.
/// Flows are processed in order of their first frame, so a key a parent
/// connection reveals in 105 Server Info is known when its transfer
/// connection is reached.
pub fn dissect_capture(path: &Path, opts: &Options) -> anyhow::Result<Capture> {
    let frames = capture::read_capture(path)?;
    let packets = capture::parse_packets(&frames);
    let ports = opts.ports;
    let mut flows = capture::tcp_flows(&packets, |p| ports.kind_of(p).is_some());
    flows.sort_by_key(|f| f.first_frame);
    let mut harvested: Vec<KeySource> = Vec::new();
    let mut connections = Vec::new();
    for (i, flow) in flows.iter().enumerate() {
        let conn = dissect_flow(i, flow, opts, &harvested);
        harvested.extend(conn.harvested_keys.iter().cloned());
        connections.push(conn);
    }
    let registrations = capture::udp_datagrams(&packets, ports.tracker)
        .into_iter()
        .map(|d| RegistrationRecord {
            frame: d.frame,
            src: d.src,
            dst: d.dst,
            registration: Registration::decode(&d.payload),
            datagram: d.payload,
        })
        .collect();
    Ok(Capture {
        connections,
        registrations,
        table_present: opts.table.is_some(),
    })
}

/// Decodes one TCP flow. `extra_keys` are tried first.
pub fn dissect_flow(
    flow_index: usize,
    flow: &Flow,
    opts: &Options,
    extra_keys: &[KeySource],
) -> Connection {
    let kind = opts
        .ports
        .kind_of(flow.server.port())
        .unwrap_or(ConnectionKind::Server);
    let mut conn = Connection {
        flow_index,
        kind,
        client: flow.client,
        server: flow.server,
        key: None,
        records: Vec::new(),
        harvested_keys: Vec::new(),
    };

    let Some((client_hello, c_off)) = decode_hello(&mut conn, &flow.c2s, Dir::ClientToServer)
    else {
        return conn;
    };
    let Some((server_hello, s_off)) = decode_hello(&mut conn, &flow.s2c, Dir::ServerToClient)
    else {
        return conn;
    };
    if let Some(role) = server_hello.role() {
        conn.kind = match role {
            Role::Client => ConnectionKind::Dcc,
            Role::Server => ConnectionKind::Server,
            Role::Tracker => ConnectionKind::Tracker,
            Role::GConsole => ConnectionKind::GConsole,
        };
    }
    let Some((client_proof, c_off)) =
        decode_proof(&mut conn, &flow.c2s, c_off, Dir::ClientToServer)
    else {
        return conn;
    };
    let Some((server_proof, s_off)) =
        decode_proof(&mut conn, &flow.s2c, s_off, Dir::ServerToClient)
    else {
        return conn;
    };

    // The client's proof is computed over the server's nonce.
    let key = extra_keys
        .iter()
        .chain(opts.candidates().iter())
        .find(|k| proof_auth(&k.key, &server_hello.nonce) == client_proof.auth)
        .cloned();
    let server_auth_ok = key
        .as_ref()
        .map(|k| proof_auth(&k.key, &client_hello.nonce) == server_proof.auth);
    let table = opts.table.as_ref();
    for r in &mut conn.records {
        if let Record::Proof {
            dir,
            proof,
            auth_ok,
            challenge_ok,
            ..
        } = r
        {
            *auth_ok = match dir {
                Dir::ClientToServer => key.as_ref().map(|_| true),
                Dir::ServerToClient => server_auth_ok,
            };
            // A proof answers the peer's challenge.
            let challenge = match dir {
                Dir::ClientToServer => server_hello.challenge,
                Dir::ServerToClient => client_hello.challenge,
            };
            *challenge_ok = table.map(|t| t.verify(challenge, &proof.response));
        }
    }
    let Some(key) = key else {
        conn.records.push(Record::Stopped {
            dir: Dir::ClientToServer,
            frame: flow.c2s.frame_at(c_off.saturating_sub(1)),
            reason: "no candidate key matches the client's proof; try --password".into(),
        });
        return conn;
    };
    conn.key = Some(key.clone());

    let cipher = SessionCipher::new(&key.key);
    decode_session(
        &mut conn,
        &cipher,
        &flow.c2s,
        c_off,
        Dir::ClientToServer,
        kind.server_recv_cap(),
    );
    decode_session(
        &mut conn,
        &cipher,
        &flow.s2c,
        s_off,
        Dir::ServerToClient,
        kind.client_recv_cap(),
    );
    conn.records.sort_by_key(|r| r.frame());
    conn
}

fn decode_hello(conn: &mut Connection, stream: &Stream, dir: Dir) -> Option<(Hello, usize)> {
    match Hello::decode(&stream.data) {
        Ok((hello, used)) => {
            conn.records.push(Record::Hello {
                dir,
                frame: stream.frame_at(0),
                hello: hello.clone(),
            });
            Some((hello, used))
        }
        Err(e) => {
            conn.records.push(Record::Stopped {
                dir,
                frame: stream.frame_at(0),
                reason: format!("hello: {}", describe_handshake_error(&e)),
            });
            None
        }
    }
}

fn describe_handshake_error(e: &HandshakeError) -> String {
    match e {
        HandshakeError::Incomplete { needed } => format!("need {needed} more bytes"),
        other => other.to_string(),
    }
}

fn decode_proof(
    conn: &mut Connection,
    stream: &Stream,
    off: usize,
    dir: Dir,
) -> Option<(Proof, usize)> {
    let Some(bytes) = stream.data.get(off..off + Proof::LEN) else {
        conn.records.push(Record::Stopped {
            dir,
            frame: stream.frame_at(off),
            reason: format!(
                "proof: need {} more bytes",
                off + Proof::LEN - stream.data.len()
            ),
        });
        return None;
    };
    let bytes: &[u8; 44] = bytes.try_into().expect("44 bytes");
    match Proof::decode(bytes) {
        Ok(proof) => {
            conn.records.push(Record::Proof {
                dir,
                frame: stream.frame_at(off),
                proof,
                auth_ok: None,
                challenge_ok: None,
            });
            Some((proof, off + Proof::LEN))
        }
        Err(e) => {
            conn.records.push(Record::Stopped {
                dir,
                frame: stream.frame_at(off),
                reason: format!("proof: {e}"),
            });
            None
        }
    }
}

fn is_data_kind(kind: u32) -> bool {
    kind == MessageType::DOWNLOAD_FILE_DATA.0 || kind == MessageType::UPLOAD_FILE_DATA.0
}

fn decode_session(
    conn: &mut Connection,
    cipher: &SessionCipher,
    stream: &Stream,
    mut off: usize,
    dir: Dir,
    cap: u64,
) {
    let data = &stream.data;
    while off < data.len() {
        let frame = stream.frame_at(off);
        let Some(block) = data.get(off..off + Header::LEN) else {
            conn.records.push(Record::Stopped {
                dir,
                frame,
                reason: format!("{} trailing bytes, less than a header", data.len() - off),
            });
            return;
        };
        let block: &[u8; 16] = block.try_into().expect("16 bytes");
        let header = match cipher.open_header_unbounded(block) {
            Ok(h) => h,
            Err(e) => {
                conn.records.push(Record::Stopped {
                    dir,
                    frame,
                    reason: format!("header: {e}"),
                });
                return;
            }
        };
        off += Header::LEN;
        if is_data_kind(header.kind) {
            conn.records.push(Record::DataHeader { dir, frame, header });
            let padded = header.padded_len();
            let available = data.len() - off;
            let taken = padded.min(available);
            if taken > 0 {
                conn.records.push(Record::Data {
                    dir,
                    frame: stream.frame_at(off),
                    len: (header.len).min(available as u64),
                });
            }
            if taken < padded {
                conn.records.push(Record::Stopped {
                    dir,
                    frame: stream.frame_at(data.len().saturating_sub(1)),
                    reason: format!("transfer data: {} of {} bytes captured", available, padded),
                });
                return;
            }
            off += taken;
            continue;
        }
        if header.len > cap || header.len >> 32 != 0 {
            conn.records.push(Record::Stopped {
                dir,
                frame,
                reason: format!(
                    "header: payload length {:#x} exceeds the cap {cap:#x}",
                    header.len
                ),
            });
            return;
        }
        let padded = header.padded_len();
        let Some(blocks) = data.get(off..off + padded) else {
            conn.records.push(Record::Stopped {
                dir,
                frame,
                reason: format!("payload: {} of {} bytes captured", data.len() - off, padded),
            });
            return;
        };
        let payload = cipher
            .open_payload(&header, blocks)
            .expect("block count checked");
        off += padded;
        let message = Message::decode(MessageType(header.kind), &payload);
        if dir == Dir::ServerToClient
            && header.kind == MessageType::SERVER_INFO.0
            && let Some(key) = session_key_of_server_info(&payload)
        {
            conn.harvested_keys.push(KeySource {
                label: format!("105 Server Info of {}", conn.client),
                key,
            });
        }
        conn.records.push(Record::Message {
            dir,
            frame,
            header,
            payload,
            message,
        });
    }
}

/// The 32-byte session key at the start of 105 Server Info, which an attached
/// transfer connection logs in with.
fn session_key_of_server_info(payload: &[u8]) -> Option<AccountKey> {
    payload
        .get(..32)
        .map(|k| AccountKey(k.try_into().expect("32 bytes")))
}
