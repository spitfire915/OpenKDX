//! Administration: 113–116, 140–143, 150–155, 170–175, 180–183.

use openkdx_proto::Message;
use openkdx_proto::msg::admin::{
    CloseConnectionMonitor, ConnectionMonitorInfo, DisconnectUser, GetConnectionMonitorInfo,
    GetServerSettings, HistoryList, OpenConnectionMonitor, SendBroadcast, ServerHistoryJoin,
    ServerHistoryLeave, ServerSettings, ServerSettingsInfo, SetServerSettings, ShutdownServer,
};
use openkdx_proto::msg::session::{ExtraServerInfo, GetExtraServerInfo, GetMyAddress, YourAddress};

use crate::session::Session;
use crate::{ClientError, Result};

/// Replace the server name in section 0 without changing the section's
/// length: the name is NUL-terminated inside a fixed buffer, so the new name
/// plus its terminator must fit in the old name's bytes plus the zero tail.
pub fn set_server_name(settings: &mut ServerSettings, name: &[u8]) -> Result<()> {
    let (offset, len) = settings.table[0];
    let start = (offset as usize)
        .checked_sub(ServerSettings::HEADER_LEN)
        .ok_or(ClientError::Protocol("server settings section 0 offset"))?;
    let end = start + len as usize;
    let bytes = settings
        .body
        .get(start..end)
        .ok_or(ClientError::Protocol("server settings section 0 range"))?;
    let mut main = openkdx_proto::msg::admin::ServerSettingsMain::decode(bytes)?;
    let room = main.server_name.len() + main.name_tail.len();
    if name.is_empty() || name.len() + 1 > room || name.len() > 0xff || name.contains(&0) {
        return Err(ClientError::Protocol("server name does not fit"));
    }
    main.server_name = name.to_vec();
    main.name_tail = vec![0; room - name.len()];
    // The byte at 0x52 is the name's length (reference administration.md,
    // confirmed on the lab: without it the server keeps the old length).
    main.ignored_52 = name.len() as u8;
    let new = main.to_bytes();
    debug_assert_eq!(new.len(), len as usize);
    settings.body[start..end].copy_from_slice(&new);
    Ok(())
}

impl Session {
    /// 170 → 171.
    pub async fn server_settings(&self) -> Result<ServerSettings> {
        let info: ServerSettingsInfo = self.connection().request_as(&GetServerSettings).await?;
        Ok(info.settings)
    }

    /// 172 → 173 or a success Status.
    pub async fn set_server_settings(&self, settings: ServerSettings) -> Result<Message> {
        self.connection()
            .request_ok(&SetServerSettings { settings })
            .await
    }

    /// 154; no reply, everyone gets 155.
    pub async fn broadcast(&self, text: &[u8]) -> Result<()> {
        if text.len() > 0x800 {
            return Err(ClientError::Protocol("broadcast longer than 0x800 bytes"));
        }
        self.connection()
            .send(&SendBroadcast {
                value_0: 0,
                value_2: 0,
                text: text.to_vec(),
            })
            .await
            .map(|_| ())
    }

    /// 150 → 1 Status (`[O disconnect-ban 82]`). `ban_seconds` 0 = no ban;
    /// `quiet` hides the notice. The server's ban list does not apply to
    /// 127.0.0.1 (reference sequences.md).
    pub async fn disconnect_user(
        &self,
        user_id: u32,
        ban_seconds: u32,
        message: &[u8],
        quiet: bool,
    ) -> Result<Message> {
        self.connection()
            .request_ok(&DisconnectUser {
                user_id,
                ignored_04: [0; 4],
                ban_seconds,
                ignored_0c: [0; 5],
                options: u8::from(quiet),
                ignored_12: [0; 2],
                ignored_15: [0; 3],
                message: message.to_vec(),
            })
            .await
    }

    /// 180 → 181; 183 records follow as events until `history_leave`.
    pub async fn history_join(&self) -> Result<HistoryList> {
        self.connection().request_as(&ServerHistoryJoin).await
    }

    /// 182; no reply.
    pub async fn history_leave(&self) -> Result<()> {
        self.connection()
            .send(&ServerHistoryLeave)
            .await
            .map(|_| ())
    }

    /// 140; no reply.
    pub async fn monitor_open(&self) -> Result<()> {
        self.connection()
            .send(&OpenConnectionMonitor)
            .await
            .map(|_| ())
    }

    /// 142 → 143.
    pub async fn monitor_info(&self) -> Result<ConnectionMonitorInfo> {
        self.connection()
            .request_as(&GetConnectionMonitorInfo)
            .await
    }

    /// 141; no reply.
    pub async fn monitor_close(&self) -> Result<()> {
        self.connection()
            .send(&CloseConnectionMonitor)
            .await
            .map(|_| ())
    }

    /// 174; no reply (the server sends 151 with the shutdown flag).
    pub async fn shutdown(&self, mode: u8, message: &[u8]) -> Result<()> {
        self.connection()
            .send(&ShutdownServer {
                ignored_00: 0,
                mode,
                ignored_02: [0; 2],
                message: message.to_vec(),
            })
            .await
            .map(|_| ())
    }

    /// 113 → 114.
    pub async fn extra_server_info(&self) -> Result<ExtraServerInfo> {
        self.connection().request_as(&GetExtraServerInfo).await
    }

    /// 115 → 116.
    pub async fn my_address(&self) -> Result<Vec<u8>> {
        let reply: YourAddress = self.connection().request_as(&GetMyAddress).await?;
        Ok(reply.address)
    }
}
