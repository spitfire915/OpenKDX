//! Tracker browse over TCP: 700–703.

use std::net::SocketAddr;
use std::time::Duration;

use openkdx_proto::handshake::Role;
use openkdx_proto::msg::tracker::{
    GetServerList, GetTrackerGroupList, GroupListEntry, ServerListEntry, TrackerGroupList,
    TrackerServerList,
};
use openkdx_proto::session::TRACKER_RECV_CAP;
use openkdx_proto::{AccountKey, ChallengeTable};
use tokio::net::TcpStream;

use crate::connection::Connection;
use crate::handshake::client_handshake;
use crate::{ClientError, Result};

/// A guest connection to a tracker (`[O tracker-basic]`: empty login).
#[derive(Debug, Clone)]
pub struct Tracker {
    conn: Connection,
}

impl Tracker {
    pub async fn connect(
        addr: SocketAddr,
        table: &ChallengeTable,
        timeout: Duration,
    ) -> Result<Tracker> {
        let mut stream = tokio::time::timeout(timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| ClientError::Timeout)??;
        let out = tokio::time::timeout(
            timeout,
            client_handshake(&mut stream, &[], &AccountKey::GUEST, table),
        )
        .await
        .map_err(|_| ClientError::Timeout)??;
        if out.server_hello.role() != Some(Role::Tracker) {
            return Err(ClientError::Protocol("the peer is not a tracker"));
        }
        let (conn, _events) =
            Connection::spawn(stream, out.read_buf, &AccountKey::GUEST, TRACKER_RECV_CAP);
        Ok(Tracker {
            conn: conn.with_timeout(timeout),
        })
    }

    /// 700 → 701.
    pub async fn groups(&self) -> Result<Vec<GroupListEntry>> {
        let list: TrackerGroupList = self.conn.request_as(&GetTrackerGroupList).await?;
        Ok(list.groups)
    }

    /// 702 → 703.
    pub async fn servers(&self, group: &[u8]) -> Result<Vec<ServerListEntry>> {
        let list: TrackerServerList = self
            .conn
            .request_as(&GetServerList {
                ignored_00: [0; 2],
                ignored_03: 0,
                group: group.to_vec(),
            })
            .await?;
        Ok(list.servers)
    }

    pub async fn close(self) {
        self.conn.close().await;
    }
}
