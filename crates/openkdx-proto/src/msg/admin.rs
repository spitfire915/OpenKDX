//! Messages 140–143 (connection monitor) and 150–191 (disconnects,
//! broadcasts, server settings, shutdown, history). See the reference spec's
//! `messages/monitoring.md` and `messages/administration.md`.

use crate::message_type::MessageType;
use crate::msg::common::{FilePath, Time1900};
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

empty_payload! {
    /// 140
    OpenConnectionMonitor = OPEN_CONNECTION_MONITOR;
    /// 141
    CloseConnectionMonitor = CLOSE_CONNECTION_MONITOR;
    /// 142: reply is 143. Does not reset the idle timer.
    GetConnectionMonitorInfo = GET_CONNECTION_MONITOR_INFO;
    /// 170: reply is 171.
    GetServerSettings = GET_SERVER_SETTINGS;
    /// 173: reply to 172.
    SetServerSettingsDone = SET_SERVER_SETTINGS_DONE;
    /// 175: needs KDXServer.lexe in the server's folder.
    UpgradeServerSoftware = UPGRADE_SERVER_SOFTWARE;
    /// 180: reply is 181, then 183 for each new record.
    ServerHistoryJoin = SERVER_HISTORY_JOIN;
    /// 182
    ServerHistoryLeave = SERVER_HISTORY_LEAVE;
}

fn sized_path(r: &mut Reader, len: usize) -> Result<FilePath, DecodeError> {
    let bytes = r.bytes(len)?;
    FilePath::decode(&bytes)
}

/// One connection or transfer in 143.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitoredConnection {
    pub user_id: u32,
    /// 0 for a connection, the transfer's ID for a transfer.
    pub transfer_id: u32,
    /// The transfer's start time in a transfer record.
    pub login_time: Time1900,
    pub time_10: Time1900,
    pub time_18: Time1900,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Bytes per second.
    pub rate: u32,
    pub seconds_left: u32,
    pub value_38: u32,
    pub state: u32,
    pub ignored_40: [u8; 2],
    pub class_index: u8,
    pub ignored_43: [u8; 29],
    pub ignored_64: [u8; 2],
    pub nick: Vec<u8>,
    pub login: Vec<u8>,
    /// The class name, or `?` if the class no longer exists.
    pub account: Vec<u8>,
    /// `ip:port` of the connection.
    pub address: Vec<u8>,
    /// The file being transferred; the root in a connection record.
    pub path: FilePath,
    pub padding: Vec<u8>,
}

impl MonitoredConnection {
    pub fn is_transfer(&self) -> bool {
        self.transfer_id != 0
    }
}

/// 143: reply to 142.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectionMonitorInfo {
    pub records: Vec<MonitoredConnection>,
}

impl Payload for ConnectionMonitorInfo {
    const KIND: MessageType = MessageType::CONNECTION_MONITOR_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let mut records = Vec::new();
        while !r.is_empty() {
            let start = r.pos();
            let user_id = r.u32()?;
            let transfer_id = r.u32()?;
            let login_time = Time1900::decode(&mut r)?;
            let time_10 = Time1900::decode(&mut r)?;
            let time_18 = Time1900::decode(&mut r)?;
            let bytes_done = r.u64()?;
            let bytes_total = r.u64()?;
            let rate = r.u32()?;
            let seconds_left = r.u32()?;
            let value_38 = r.u32()?;
            let state = r.u32()?;
            let ignored_40: [u8; 2] = r.array()?;
            let class_index = r.u8()?;
            let ignored_43: [u8; 29] = r.array()?;
            let nick_len = usize::from(r.u8()?);
            let login_len = usize::from(r.u8()?);
            let account_len = usize::from(r.u8()?);
            let address_len = usize::from(r.u8()?);
            let ignored_64: [u8; 2] = r.array()?;
            let path_len = usize::from(r.u16()?);
            records.push(MonitoredConnection {
                user_id,
                transfer_id,
                login_time,
                time_10,
                time_18,
                bytes_done,
                bytes_total,
                rate,
                seconds_left,
                value_38,
                state,
                ignored_40,
                class_index,
                ignored_43,
                ignored_64,
                nick: r.bytes(nick_len)?,
                login: r.bytes(login_len)?,
                account: r.bytes(account_len)?,
                address: r.bytes(address_len)?,
                path: sized_path(&mut r, path_len)?,
                padding: r.pad4(start)?,
            });
        }
        Ok(ConnectionMonitorInfo { records })
    }

    fn encode(&self, w: &mut Writer) {
        for m in &self.records {
            let start = w.pos();
            let path = m.path.to_bytes();
            w.u32(m.user_id);
            w.u32(m.transfer_id);
            m.login_time.encode(w);
            m.time_10.encode(w);
            m.time_18.encode(w);
            w.u64(m.bytes_done);
            w.u64(m.bytes_total);
            w.u32(m.rate);
            w.u32(m.seconds_left);
            w.u32(m.value_38);
            w.u32(m.state);
            w.bytes(&m.ignored_40);
            w.u8(m.class_index);
            w.bytes(&m.ignored_43);
            w.u8(m.nick.len() as u8);
            w.u8(m.login.len() as u8);
            w.u8(m.account.len() as u8);
            w.u8(m.address.len() as u8);
            w.bytes(&m.ignored_64);
            w.u16(path.len() as u16);
            w.bytes(&m.nick);
            w.bytes(&m.login);
            w.bytes(&m.account);
            w.bytes(&m.address);
            w.bytes(&path);
            w.pad4(start, &m.padding);
        }
    }
}

/// 150: needs Disconnect Users, and Ban Users to ban.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DisconnectUser {
    pub user_id: u32,
    pub ignored_04: [u8; 4],
    /// At most 0x80000; 0 for no ban.
    pub ban_seconds: u32,
    pub ignored_0c: [u8; 5],
    /// Bit 0: disconnect without a notice.
    pub options: u8,
    pub ignored_12: [u8; 2],
    pub ignored_15: [u8; 3],
    pub message: Vec<u8>,
}

impl Payload for DisconnectUser {
    const KIND: MessageType = MessageType::DISCONNECT_USER;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let user_id = r.u32()?;
        let ignored_04: [u8; 4] = r.array()?;
        let ban_seconds = r.u32()?;
        let ignored_0c: [u8; 5] = r.array()?;
        let options = r.u8()?;
        let ignored_12: [u8; 2] = r.array()?;
        let message_len = usize::from(r.u8()?);
        let ignored_15: [u8; 3] = r.array()?;
        let message = r.bytes(message_len)?;
        r.finish()?;
        Ok(DisconnectUser {
            user_id,
            ignored_04,
            ban_seconds,
            ignored_0c,
            options,
            ignored_12,
            ignored_15,
            message,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        w.bytes(&self.ignored_04);
        w.u32(self.ban_seconds);
        w.bytes(&self.ignored_0c);
        w.u8(self.options);
        w.bytes(&self.ignored_12);
        w.u8(self.message.len() as u8);
        w.bytes(&self.ignored_15);
        w.bytes(&self.message);
    }
}

/// 151: to the target of 150, and broadcast by 174 and 175.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DisconnectMessage {
    pub user_id: u32,
    pub ignored: [u8; 8],
    /// 2: banned. 4: the server is shutting down or upgrading.
    pub flags: u16,
    pub value_0e: u16,
    pub value_10: u16,
    pub message: Vec<u8>,
    pub name: Vec<u8>,
}

impl DisconnectMessage {
    pub const FLAG_BANNED: u16 = 2;
    pub const FLAG_SHUTDOWN: u16 = 4;
}

impl Payload for DisconnectMessage {
    const KIND: MessageType = MessageType::DISCONNECT_MESSAGE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let user_id = r.u32()?;
        let ignored: [u8; 8] = r.array()?;
        let flags = r.u16()?;
        let value_0e = r.u16()?;
        let value_10 = r.u16()?;
        let message_len = usize::from(r.u8()?);
        let name_len = usize::from(r.u8()?);
        let out = DisconnectMessage {
            user_id,
            ignored,
            flags,
            value_0e,
            value_10,
            message: r.bytes(message_len)?,
            name: r.bytes(name_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        w.bytes(&self.ignored);
        w.u16(self.flags);
        w.u16(self.value_0e);
        w.u16(self.value_10);
        w.u8(self.message.len() as u8);
        w.u8(self.name.len() as u8);
        w.bytes(&self.message);
        w.bytes(&self.name);
    }
}

/// 152: reply is 153.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetUserInfo {
    pub user_id: u32,
}

impl Payload for GetUserInfo {
    const KIND: MessageType = MessageType::GET_USER_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let user_id = r.u32()?;
        r.finish()?;
        Ok(GetUserInfo { user_id })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
    }
}

/// 153: reply to 152.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    pub user_id: u32,
    pub ignored_04: [u8; 4],
    pub login_time: Time1900,
    pub time_10: Time1900,
    pub time_18: Time1900,
    pub ignored_20: [u8; 27],
    pub ignored_3c: u8,
    pub ignored_3e: u8,
    pub ignored_42: [u8; 2],
    pub address: Vec<u8>,
    pub nick: Vec<u8>,
    /// Probably the class or account name.
    pub name_3f: Vec<u8>,
    pub login: Vec<u8>,
    pub name_41: Vec<u8>,
    /// Uninitialized on the wire.
    pub padding: Vec<u8>,
}

impl Payload for UserInfo {
    const KIND: MessageType = MessageType::USER_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let user_id = r.u32()?;
        let ignored_04: [u8; 4] = r.array()?;
        let login_time = Time1900::decode(&mut r)?;
        let time_10 = Time1900::decode(&mut r)?;
        let time_18 = Time1900::decode(&mut r)?;
        let ignored_20: [u8; 27] = r.array()?;
        let address_len = usize::from(r.u8()?);
        let ignored_3c = r.u8()?;
        let nick_len = usize::from(r.u8()?);
        let ignored_3e = r.u8()?;
        let name_3f_len = usize::from(r.u8()?);
        let login_len = usize::from(r.u8()?);
        let name_41_len = usize::from(r.u8()?);
        let ignored_42: [u8; 2] = r.array()?;
        let out = UserInfo {
            user_id,
            ignored_04,
            login_time,
            time_10,
            time_18,
            ignored_20,
            ignored_3c,
            ignored_3e,
            ignored_42,
            address: r.bytes(address_len)?,
            nick: r.bytes(nick_len)?,
            name_3f: r.bytes(name_3f_len)?,
            login: r.bytes(login_len)?,
            name_41: r.bytes(name_41_len)?,
            padding: r.pad4_lenient(0)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        w.bytes(&self.ignored_04);
        self.login_time.encode(w);
        self.time_10.encode(w);
        self.time_18.encode(w);
        w.bytes(&self.ignored_20);
        w.u8(self.address.len() as u8);
        w.u8(self.ignored_3c);
        w.u8(self.nick.len() as u8);
        w.u8(self.ignored_3e);
        w.u8(self.name_3f.len() as u8);
        w.u8(self.login.len() as u8);
        w.u8(self.name_41.len() as u8);
        w.bytes(&self.ignored_42);
        w.bytes(&self.address);
        w.bytes(&self.nick);
        w.bytes(&self.name_3f);
        w.bytes(&self.login);
        w.bytes(&self.name_41);
        if !self.padding.is_empty() {
            w.pad4(0, &self.padding);
        }
    }
}

/// 154: needs Broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SendBroadcast {
    pub value_0: u16,
    /// Copied to 155's `value_0e`.
    pub value_2: u16,
    /// At most 0x800 bytes.
    pub text: Vec<u8>,
}

impl Payload for SendBroadcast {
    const KIND: MessageType = MessageType::SEND_BROADCAST_MESSAGE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Ok(SendBroadcast {
            value_0: r.u16()?,
            value_2: r.u16()?,
            text: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u16(self.value_0);
        w.u16(self.value_2);
        w.bytes(&self.text);
    }
}

/// 155: a broadcast; `user_id` 0 for the server's own notices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RcvBroadcast {
    pub user_id: u32,
    pub ignored: [u8; 10],
    pub value_0e: u16,
    pub text: Vec<u8>,
}

impl Payload for RcvBroadcast {
    const KIND: MessageType = MessageType::RCV_BROADCAST_MESSAGE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Ok(RcvBroadcast {
            user_id: r.u32()?,
            ignored: r.array()?,
            value_0e: r.u16()?,
            text: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        w.bytes(&self.ignored);
        w.u16(self.value_0e);
        w.bytes(&self.text);
    }
}

/// The server's settings blob, as 171 and 172 carry it: `%KDX` `SSTG`, a
/// version, a table of 11 sections, 12 flag bytes, and the sections' bytes.
/// Entry 0 is the main section ([`ServerSettingsMain`]); the others are kept
/// as bytes until the server (Phase 4) needs them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    pub version: u16,
    pub ignored: [u8; 2],
    /// `(offset from the start of the blob, length)` per section.
    pub table: [(u32, u32); 11],
    /// `01 01 01 01 00 01 00 02 00 00 00 00` in a 171.
    pub value_64: [u8; 12],
    /// Everything after the 0x70-byte header.
    pub body: Vec<u8>,
}

impl ServerSettings {
    pub const HEADER_LEN: usize = 0x70;

    /// The bytes of section `i`, or `None` if it is empty or out of range.
    pub fn section(&self, i: usize) -> Option<&[u8]> {
        let (offset, len) = *self.table.get(i)?;
        if len == 0 {
            return None;
        }
        let start = (offset as usize).checked_sub(Self::HEADER_LEN)?;
        self.body.get(start..start.checked_add(len as usize)?)
    }

    pub fn main(&self) -> Option<ServerSettingsMain> {
        ServerSettingsMain::decode(self.section(0)?).ok()
    }

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        if r.array::<4>()? != *b"%KDX" || r.array::<4>()? != *b"SSTG" {
            return Err(DecodeError::Invalid("server settings magic"));
        }
        let version = r.u16()?;
        let ignored: [u8; 2] = r.array()?;
        let mut table = [(0u32, 0u32); 11];
        for entry in &mut table {
            *entry = (r.u32()?, r.u32()?);
        }
        let value_64: [u8; 12] = r.array()?;
        Ok(ServerSettings {
            version,
            ignored,
            table,
            value_64,
            body: r.rest(),
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        w.bytes(b"%KDX");
        w.bytes(b"SSTG");
        w.u16(self.version);
        w.bytes(&self.ignored);
        for (offset, len) in &self.table {
            w.u32(*offset);
            w.u32(*len);
        }
        w.bytes(&self.value_64);
        w.bytes(&self.body);
    }
}

/// Section 0 of the settings blob, as far as it is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettingsMain {
    pub ignored_00: [u8; 35],
    /// Bit 1: the greeting is on.
    pub flags: u8,
    pub ignored_24: [u8; 20],
    /// Bytes per second; 0 when off.
    pub total_outgoing_speed_limit: u32,
    pub ignored_3c: [u8; 22],
    pub ignored_52: u8,
    pub ignored_53: [u8; 9],
    /// NUL-terminated on the wire; the terminator and anything after it are
    /// in `name_tail`.
    pub server_name: Vec<u8>,
    pub name_tail: Vec<u8>,
}

impl ServerSettingsMain {
    pub fn greeting_enabled(&self) -> bool {
        self.flags & 2 != 0
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 35] = r.array()?;
        let flags = r.u8()?;
        let ignored_24: [u8; 20] = r.array()?;
        let total_outgoing_speed_limit = r.u32()?;
        let ignored_3c: [u8; 22] = r.array()?;
        let ignored_52 = r.u8()?;
        let ignored_53: [u8; 9] = r.array()?;
        let rest = r.rest();
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        Ok(ServerSettingsMain {
            ignored_00,
            flags,
            ignored_24,
            total_outgoing_speed_limit,
            ignored_3c,
            ignored_52,
            ignored_53,
            server_name: rest[..end].to_vec(),
            name_tail: rest[end..].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.bytes(&self.ignored_00);
        w.u8(self.flags);
        w.bytes(&self.ignored_24);
        w.u32(self.total_outgoing_speed_limit);
        w.bytes(&self.ignored_3c);
        w.u8(self.ignored_52);
        w.bytes(&self.ignored_53);
        w.bytes(&self.server_name);
        w.bytes(&self.name_tail);
        w.into_inner()
    }
}

macro_rules! server_settings_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name {
                pub settings: ServerSettings,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let settings = ServerSettings::decode_from(&mut r)?;
                    Ok($name { settings })
                }

                fn encode(&self, w: &mut Writer) {
                    self.settings.encode_into(w);
                }
            }
        )*
    };
}

server_settings_payload! {
    /// 171: reply to 170.
    ServerSettingsInfo = SERVER_SETTINGS_INFO;
    /// 172: reply is 173.
    SetServerSettings = SET_SERVER_SETTINGS;
}

/// 174: needs Shutdown Server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShutdownServer {
    pub ignored_00: u8,
    /// The low 3 bits must not all be 0.
    pub mode: u8,
    pub ignored_02: [u8; 2],
    /// At most 0xff bytes; no length byte.
    pub message: Vec<u8>,
}

impl Payload for ShutdownServer {
    const KIND: MessageType = MessageType::SHUTDOWN_SERVER;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Ok(ShutdownServer {
            ignored_00: r.u8()?,
            mode: r.u8()?,
            ignored_02: r.array()?,
            message: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u8(self.ignored_00);
        w.u8(self.mode);
        w.bytes(&self.ignored_02);
        w.bytes(&self.message);
    }
}

/// The kind of a history record: message types 184 to 191.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HistoryKind(pub u32);

impl HistoryKind {
    pub const LOGIN: HistoryKind = HistoryKind(184);
    pub const DISCONNECTED: HistoryKind = HistoryKind(185);
    pub const ABORT_DOWNLOAD: HistoryKind = HistoryKind(186);
    pub const ABORT_UPLOAD: HistoryKind = HistoryKind(187);
    pub const FINISHED_DOWNLOAD: HistoryKind = HistoryKind(188);
    pub const FINISHED_UPLOAD: HistoryKind = HistoryKind(189);
    pub const SERVER_STARTED: HistoryKind = HistoryKind(190);
    pub const LOGIN_FAILED: HistoryKind = HistoryKind(191);

    pub fn name(self) -> Option<&'static str> {
        if (184..=191).contains(&self.0) {
            MessageType(self.0).name()
        } else {
            None
        }
    }
}

/// One record in 181 and 183.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRecord {
    pub time: Time1900,
    pub kind: HistoryKind,
    /// 0 for Server Started.
    pub user_id: u32,
    pub ignored_10: [u8; 50],
    pub ignored_44: [u8; 2],
    pub ignored_4a: [u8; 2],
    pub address: Vec<u8>,
    /// The nick before the event.
    pub nick: Vec<u8>,
    pub login: Vec<u8>,
    /// The new nick for a 100, the chat name for a 202.
    pub detail: Vec<u8>,
    pub path: FilePath,
    pub padding: Vec<u8>,
}

impl HistoryRecord {
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let time = Time1900::decode(r)?;
        let kind = HistoryKind(r.u32()?);
        let user_id = r.u32()?;
        let ignored_10: [u8; 50] = r.array()?;
        let path_len = usize::from(r.u16()?);
        let ignored_44: [u8; 2] = r.array()?;
        let address_len = usize::from(r.u8()?);
        let nick_len = usize::from(r.u8()?);
        let login_len = usize::from(r.u8()?);
        let detail_len = usize::from(r.u8()?);
        let ignored_4a: [u8; 2] = r.array()?;
        Ok(HistoryRecord {
            time,
            kind,
            user_id,
            ignored_10,
            ignored_44,
            ignored_4a,
            address: r.bytes(address_len)?,
            nick: r.bytes(nick_len)?,
            login: r.bytes(login_len)?,
            detail: r.bytes(detail_len)?,
            path: sized_path(r, path_len)?,
            padding: r.pad4_lenient(start)?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        let path = self.path.to_bytes();
        self.time.encode(w);
        w.u32(self.kind.0);
        w.u32(self.user_id);
        w.bytes(&self.ignored_10);
        w.u16(path.len() as u16);
        w.bytes(&self.ignored_44);
        w.u8(self.address.len() as u8);
        w.u8(self.nick.len() as u8);
        w.u8(self.login.len() as u8);
        w.u8(self.detail.len() as u8);
        w.bytes(&self.ignored_4a);
        w.bytes(&self.address);
        w.bytes(&self.nick);
        w.bytes(&self.login);
        w.bytes(&self.detail);
        w.bytes(&path);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

/// 181: reply to 180.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryList {
    pub ignored: u32,
    pub records: Vec<HistoryRecord>,
}

impl Payload for HistoryList {
    const KIND: MessageType = MessageType::SERVER_HISTORY_JOIN_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored = r.u32()?;
        let count = r.u32()? as usize;
        let mut records = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            records.push(HistoryRecord::decode(&mut r)?);
        }
        r.finish()?;
        Ok(HistoryList { ignored, records })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.ignored);
        w.u32(self.records.len() as u32);
        for rec in &self.records {
            rec.encode(w);
        }
    }
}

/// 183: a new record while joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryAdded {
    pub record: HistoryRecord,
}

impl Payload for HistoryAdded {
    const KIND: MessageType = MessageType::SERVER_HISTORY_ADDED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let record = HistoryRecord::decode(&mut r)?;
        r.finish()?;
        Ok(HistoryAdded { record })
    }

    fn encode(&self, w: &mut Writer) {
        self.record.encode(w);
    }
}
