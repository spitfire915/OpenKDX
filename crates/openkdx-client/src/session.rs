//! A logged-in connection to a KDX server.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use openkdx_proto::msg::common::UserRecord;
use openkdx_proto::msg::session::{LicenseReport, Ping, Pong, ServerInfo, SetIdentity};
use openkdx_proto::session::CLIENT_RECV_CAP;
use openkdx_proto::{AccountKey, ChallengeTable, Message, MessageType, Payload};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

use crate::connection::{Connection, Event};
use crate::handshake::client_handshake;
use crate::{ClientError, Identity, Result, load_kdx_table};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    /// Empty for guest.
    pub login: Vec<u8>,
    /// Empty for guest or a password-less account.
    pub password: Vec<u8>,
}

impl Credentials {
    pub fn guest() -> Self {
        Credentials {
            login: Vec::new(),
            password: Vec::new(),
        }
    }

    pub fn account_key(&self) -> AccountKey {
        AccountKey::from_password(&self.password)
    }
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub identity: Identity,
    /// Request timeout, and the time allowed for the login replies.
    pub timeout: Duration,
    /// `None` loads the table from the environment or the cache.
    pub table: Option<ChallengeTable>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        ConnectOptions {
            identity: Identity::default(),
            timeout: Duration::from_secs(30),
            table: None,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    users: Vec<UserRecord>,
    greeting: Option<Vec<u8>>,
    must_join_chat: bool,
}

impl State {
    fn apply(&mut self, message: &Message) {
        match message {
            Message::UserList(l) => self.users = l.users.clone(),
            Message::UserJoined(j) => self.upsert(&j.user),
            Message::UserChanged(c) => self.upsert(&c.user),
            Message::UserDeparted(d) => self.users.retain(|u| u.user_id != d.user_id),
            Message::ServerGreeting(g) => self.greeting = Some(g.text.clone()),
            Message::MustJoinChat(_) => self.must_join_chat = true,
            _ => {}
        }
    }

    fn upsert(&mut self, user: &UserRecord) {
        match self.users.iter_mut().find(|u| u.user_id == user.user_id) {
            Some(slot) => *slot = user.clone(),
            None => self.users.push(user.clone()),
        }
    }
}

/// A connection that has logged in: 100 sent, 101 and 105 received.
#[derive(Debug, Clone)]
pub struct Session {
    conn: Connection,
    pub server: ServerInfo,
    pub server_addr: SocketAddr,
    pub login: Vec<u8>,
    pub identity: Identity,
    table: ChallengeTable,
    state: Arc<Mutex<State>>,
}

impl Session {
    pub async fn connect(
        addr: SocketAddr,
        creds: &Credentials,
        opts: ConnectOptions,
    ) -> Result<Session> {
        let table = match opts.table {
            Some(t) => t,
            None => load_kdx_table()?,
        };
        let key = creds.account_key();
        let mut stream = tokio::time::timeout(opts.timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| ClientError::Timeout)??;
        let out = tokio::time::timeout(
            opts.timeout,
            client_handshake(&mut stream, &creds.login, &key, &table),
        )
        .await
        .map_err(|_| ClientError::Timeout)??;
        let (conn, mut events) = Connection::spawn(stream, out.read_buf, &key, CLIENT_RECV_CAP);
        let conn = conn.with_timeout(opts.timeout);
        // The login replies get the full timeout from here, after connect
        // and handshake each had their own.
        let deadline = Instant::now() + opts.timeout;

        conn.send(&SetIdentity {
            user: opts.identity.user_record(),
        })
        .await?;
        if opts.identity.registered {
            conn.send(&LicenseReport { value: 1 }).await?;
        }

        let state = Arc::new(Mutex::new(State::default()));
        let mut server = None;
        let mut have_users = false;
        while server.is_none() || !have_users {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(ClientError::Timeout)?;
            let event = tokio::time::timeout(remaining, events.recv())
                .await
                .map_err(|_| ClientError::Timeout)?;
            match event {
                Ok(Event::Message(m)) => {
                    match &*m {
                        Message::ServerInfo(info) => server = Some(info.clone()),
                        Message::UserList(_) => have_users = true,
                        Message::Status(s) if s.is_error() => {
                            return Err(ClientError::Server {
                                code: s.code,
                                name: openkdx_proto::msg::error_name(s.code),
                                request: SetIdentity::KIND.0,
                            });
                        }
                        _ => {}
                    }
                    state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .apply(&m);
                }
                Ok(Event::Undecodable { kind, error, .. })
                    if kind == MessageType::USER_LIST || kind == MessageType::SERVER_INFO =>
                {
                    return Err(ClientError::Decode(error));
                }
                Ok(Event::Undecodable { .. }) => {}
                Ok(Event::Disconnected(d)) => {
                    return Err(match d.message {
                        Some(m) => ClientError::DisconnectedBy(Box::new(m)),
                        None => ClientError::Disconnected,
                    });
                }
                Err(broadcast::error::RecvError::Closed) => return Err(ClientError::Disconnected),
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            }
        }

        let tracker_state = state.clone();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(Event::Message(m)) => tracker_state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .apply(&m),
                    Ok(Event::Disconnected(_)) | Err(broadcast::error::RecvError::Closed) => break,
                    Ok(Event::Undecodable { .. }) | Err(broadcast::error::RecvError::Lagged(_)) => {
                    }
                }
            }
        });

        Ok(Session {
            conn,
            server: server.expect("set by the loop"),
            server_addr: addr,
            login: creds.login.clone(),
            identity: opts.identity,
            table,
            state,
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// The request timeout, also used when waiting for an unsolicited
    /// confirmation.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.conn = self.conn.clone().with_timeout(timeout);
    }

    /// Wait up to the timeout for the first message `pick` accepts. The
    /// receiver must have been subscribed before the request went out.
    pub async fn wait_for<T>(
        &self,
        events: &mut broadcast::Receiver<Event>,
        mut pick: impl FnMut(&Message) -> Option<T>,
    ) -> Result<T> {
        let deadline = Instant::now() + self.conn.timeout();
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(ClientError::Timeout)?;
            match tokio::time::timeout(remaining, events.recv()).await {
                Err(_) => return Err(ClientError::Timeout),
                Ok(Ok(Event::Message(m))) => {
                    if let Some(v) = pick(&m) {
                        return Ok(v);
                    }
                }
                Ok(Ok(Event::Undecodable { .. })) => {}
                Ok(Ok(Event::Disconnected(d))) => {
                    return Err(match d.message {
                        Some(m) => ClientError::DisconnectedBy(Box::new(m)),
                        None => ClientError::Disconnected,
                    });
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(ClientError::Disconnected);
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {}
            }
        }
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.conn.events()
    }

    pub fn user_id(&self) -> u32 {
        self.server.user_id
    }

    /// The key an attached transfer connection logs in with.
    pub fn session_key(&self) -> AccountKey {
        AccountKey(self.server.session_key)
    }

    /// Where transfer connections go: the File Transfer port if the server
    /// names one, else the main port.
    pub fn transfer_addr(&self) -> SocketAddr {
        let mut addr = self.server_addr;
        if self.server.file_transfer_port != 0
            && let Ok(port) = u16::try_from(self.server.file_transfer_port)
        {
            addr.set_port(port);
        }
        addr
    }

    pub fn table(&self) -> &ChallengeTable {
        &self.table
    }

    pub fn users(&self) -> Vec<UserRecord> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .users
            .clone()
    }

    pub fn greeting(&self) -> Option<Vec<u8>> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .greeting
            .clone()
    }

    pub fn must_join_chat(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .must_join_chat
    }

    pub async fn set_identity(&self, identity: &Identity) -> Result<()> {
        self.conn
            .send(&SetIdentity {
                user: identity.user_record(),
            })
            .await
            .map(|_| ())
    }

    pub async fn ping(&self) -> Result<()> {
        self.conn.request_as::<_, Pong>(&Ping).await.map(|_| ())
    }

    pub async fn close(self) {
        self.conn.close().await;
    }
}
