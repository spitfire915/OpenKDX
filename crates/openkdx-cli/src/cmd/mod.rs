pub mod session;

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::{Context as _, anyhow};
use openkdx_client::session::{ConnectOptions, Credentials, Session};
use openkdx_client::{Identity, Result as ClientResult};
use openkdx_proto::ChallengeTable;

use crate::args::{Command, Globals};
use crate::out::Out;

/// What every command shares: the options, the output, and the one session
/// (opened on first use so `run` can reuse it).
pub struct Ctx {
    pub globals: Globals,
    pub out: Out,
    session: Option<Session>,
}

impl Ctx {
    pub fn new(globals: Globals) -> Self {
        let out = Out {
            json: globals.json,
            quiet: globals.quiet,
        };
        Ctx {
            globals,
            out,
            session: None,
        }
    }

    pub fn server_addr(&self) -> anyhow::Result<SocketAddr> {
        resolve(&self.globals.server, 10700)
    }

    pub fn table(&self) -> ClientResult<ChallengeTable> {
        if self.globals.insecure_default_table {
            Ok(ChallengeTable::default_table())
        } else {
            openkdx_client::load_kdx_table()
        }
    }

    pub fn connect_options(&self) -> ClientResult<ConnectOptions> {
        Ok(ConnectOptions {
            identity: Identity {
                nick: self.globals.nick.as_bytes().to_vec(),
                ..Identity::default()
            },
            timeout: self.globals.timeout(),
            table: Some(self.table()?),
        })
    }

    pub fn credentials(&self) -> Credentials {
        Credentials {
            login: self.globals.login.as_bytes().to_vec(),
            password: self.globals.password.as_bytes().to_vec(),
        }
    }

    /// The session, logging in on first use.
    pub async fn session(&mut self) -> anyhow::Result<&Session> {
        if self.session.is_none() {
            let addr = self.server_addr()?;
            let session =
                Session::connect(addr, &self.credentials(), self.connect_options()?).await?;
            self.session = Some(session);
        }
        Ok(self.session.as_ref().expect("just set"))
    }

    pub async fn close(&mut self) {
        if let Some(s) = self.session.take() {
            s.close().await;
        }
    }
}

/// `HOST`, `HOST:PORT`, or `[v6]:PORT` to one address.
pub fn resolve(spec: &str, default_port: u16) -> anyhow::Result<SocketAddr> {
    if let Ok(addr) = spec.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let with_port: String = if spec.contains(':') && !spec.starts_with('[') {
        spec.to_string()
    } else {
        format!("{spec}:{default_port}")
    };
    with_port
        .to_socket_addrs()
        .with_context(|| format!("cannot resolve {spec}"))?
        .find(|a| a.is_ipv4())
        .or_else(|| with_port.to_socket_addrs().ok()?.next())
        .ok_or_else(|| anyhow!("no address for {spec}"))
}

pub async fn dispatch(ctx: &mut Ctx, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Login { stay } => session::login(ctx, stay).await,
        Command::Users => session::users(ctx).await,
        Command::Run { keep_going } => session::run(ctx, keep_going).await,
    }
}
