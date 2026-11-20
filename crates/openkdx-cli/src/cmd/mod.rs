pub mod accounts;
pub mod admin;
pub mod chat;
pub mod files;
pub mod news;
pub mod session;
pub mod tracker;

use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, anyhow};
use kdx_client::{Client, Login};
use kdx_proto::ChallengeTable;
use kdx_proto::handshake::HandshakeError;
use kdx_proto::msg::{FilePath, UserRecord};
use openkdx_table::{Cache, Story};

use crate::args::{Command, Globals};
use crate::out::Out;

/// No challenge table could be loaded; the message says where to get one.
#[derive(Debug, thiserror::Error)]
#[error(
    "no KDX challenge table: {source}; run `openkdx-table fetch` once, or set OPENKDX_KDX_TABLE"
)]
pub struct NoTable {
    #[source]
    source: openkdx_table::TableError,
}

/// The server closed the connection, with its 151 message when it sent one.
#[derive(Debug, thiserror::Error)]
pub enum Disconnected {
    #[error("disconnected by the server: {0}")]
    By(String),
    #[error("disconnected")]
    Gone,
}

/// What every command shares: the options, the output, and the one client
/// (logged in on first use so `run` can reuse it).
pub struct Ctx {
    pub globals: Globals,
    pub out: Out,
    client: Option<Client>,
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
            client: None,
        }
    }

    pub fn server_addr(&self) -> anyhow::Result<SocketAddr> {
        resolve(&self.globals.server, 10700)
    }

    /// The KDX challenge table: the placeholder under
    /// `--insecure-default-table`, else `openkdx_table::load`.
    pub fn table(&self) -> anyhow::Result<ChallengeTable> {
        if self.globals.insecure_default_table {
            return Ok(ChallengeTable::default());
        }
        let loaded = match &self.globals.table_cache {
            Some(dir) => openkdx_table::load_from(Story::Kdx, &Cache::new(PathBuf::from(dir))),
            None => openkdx_table::load(Story::Kdx),
        };
        loaded.map_err(|source| NoTable { source }.into())
    }

    pub fn login(&self) -> Login {
        Login {
            name: self.globals.login.as_bytes().to_vec(),
            password: self.globals.password.as_bytes().to_vec(),
        }
    }

    pub fn identity(&self) -> UserRecord {
        UserRecord {
            nick: self.globals.nick.as_bytes().to_vec(),
            ..UserRecord::default()
        }
    }

    pub fn timeout(&self) -> Duration {
        self.globals.timeout()
    }

    /// The client, logging in on first use.
    pub async fn client(&mut self) -> anyhow::Result<&mut Client> {
        if self.client.is_none() {
            let addr = self.server_addr()?;
            let table = self.table()?;
            let login = self.login();
            let me = self.identity();
            let client =
                tokio::time::timeout(self.timeout(), Client::login(addr, &login, &me, table))
                    .await
                    .map_err(anyhow::Error::from)?
                    .map_err(login_error)?;
            self.client = Some(client);
        }
        Ok(self.client.as_mut().expect("just set"))
    }

    pub async fn close(&mut self) {
        // Dropping the client closes the socket; the originals need nothing more.
        self.client.take();
    }
}

/// Runs one request under the `--timeout`; running out is exit code 4.
pub async fn within<T>(
    timeout: Duration,
    fut: impl Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    tokio::time::timeout(timeout, fut)
        .await
        .map_err(anyhow::Error::from)?
}

/// A failed proof reads as "login failed"; the typed error stays in the
/// chain so the exit code can find it.
fn login_error(err: anyhow::Error) -> anyhow::Error {
    match err.downcast_ref::<HandshakeError>() {
        Some(_) => err.context("login failed"),
        None => err,
    }
}

/// A user named by ID or nick.
pub fn find_user(client: &Client, spec: &str) -> anyhow::Result<u32> {
    if let Ok(id) = spec.parse::<u32>() {
        return Ok(id);
    }
    client
        .users
        .iter()
        .find(|u| u.nick == spec.as_bytes())
        .map(|u| u.user_id)
        .ok_or_else(|| anyhow!("no user named {spec}"))
}

/// `a/b/c` to a path; empty segments are skipped, so `""` is the base.
pub fn parse_path(s: &str) -> FilePath {
    FilePath::new(s.split('/').filter(|seg| !seg.is_empty()))
}

/// Names joined by `/`, lossily decoded.
pub fn path_string(path: &FilePath) -> String {
    path.0
        .iter()
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn join_path(parent: &FilePath, name: &[u8]) -> FilePath {
    let mut names = parent.0.clone();
    names.push(name.to_vec());
    FilePath(names)
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
        Command::Chat(c) => chat::chat(ctx, c).await,
        Command::Pm(c) => chat::pm(ctx, c).await,
        Command::News(c) => news::news(ctx, c).await,
        Command::Files(c) => files::files(ctx, c).await,
        Command::Accounts(c) => accounts::accounts(ctx, c).await,
        Command::Classes(c) => accounts::classes(ctx, c).await,
        Command::FolderAccess(c) => accounts::folder_access(ctx, c).await,
        Command::Server(c) => admin::server(ctx, c).await,
        Command::Tracker { tracker, command } => tracker::tracker(ctx, &tracker, command).await,
    }
}

#[cfg(test)]
mod tests {
    use super::{join_path, parse_path, path_string};

    #[test]
    fn parse_path_skips_empty_segments_and_prints_back() {
        let p = parse_path("/Uploads//a b/");
        assert_eq!(p.0, vec![b"Uploads".to_vec(), b"a b".to_vec()]);
        assert_eq!(path_string(&p), "Uploads/a b");
        assert!(parse_path("").0.is_empty());
        assert_eq!(path_string(&join_path(&p, b"c")), "Uploads/a b/c");
    }
}
