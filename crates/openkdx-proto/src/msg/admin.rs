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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

/// The server's settings, as 171 and 172 carry them and as `KDXServer.stg`
/// stores them: `%KDX` `SSTG`, a version, a table of 11 `(offset, length)`
/// section entries, 12 bytes, then the sections at 4-aligned offsets.
///
/// Sections, pinned by diffing the lab server's file after one change each
/// (`docs/formats/server-stg.md`): 0 main, 1 trackers, 2 allow/deny list,
/// 3 greeting (107 layout), 5 icon (106 bytes), 7 chat rooms (kept as bytes
/// until Phase 4). Sections 4, 6, 8–10 have never been seen non-empty and
/// are kept as bytes too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    pub version: u16,
    pub ignored: [u8; 2],
    /// `01 01 01 01 00 01 00 02 00 00 00` in the file and a 171. In a 172,
    /// 0x6b is 0, and so is 0x69 without the icon. The twelfth byte, at
    /// 0x6f, is the tracker count and is derived from `trackers`.
    pub value_64: [u8; 11],
    /// Whether an empty section still gets the running offset in the table
    /// (the server does this for 1, 2, 3, 5 and 7) or `(0, 0)` (4, 6, 8–10;
    /// a client's 172 also sends `(0, 0)` for 7).
    pub placeholders: [bool; 11],
    pub main: ServerSettingsMain,
    pub trackers: Vec<TrackerEntry>,
    pub allow_deny: Vec<AddressRule>,
    pub greeting: Option<Greeting>,
    /// The server icon bitmap, as a 106 carries it.
    pub icon: Option<Vec<u8>>,
    /// Section 7: the chat rooms; modelled in Phase 4.
    pub chats: Vec<u8>,
    /// Sections 4, 6, 8, 9, 10 (indexed by section number; the others stay
    /// empty here).
    pub other: [Vec<u8>; 11],
    /// Bytes after the last section's padding; never seen non-empty.
    pub trailing: Vec<u8>,
}

/// Section 3: four bytes, then the greeting text (the 107 layout).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Greeting {
    pub ignored: [u8; 4],
    pub text: Vec<u8>,
}

impl ServerSettings {
    pub const HEADER_LEN: usize = 0x70;
    pub const SECTION_MAIN: usize = 0;
    pub const SECTION_TRACKERS: usize = 1;
    pub const SECTION_ALLOW_DENY: usize = 2;
    pub const SECTION_GREETING: usize = 3;
    pub const SECTION_ICON: usize = 5;
    pub const SECTION_CHATS: usize = 7;
    const SERVER_PLACEHOLDERS: [bool; 11] = [
        true, true, true, true, false, true, false, true, false, false, false,
    ];

    /// Decodes a whole blob.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Self::decode_from(&mut r)
    }

    /// Every string must fit its length byte. The encoder truncates to keep
    /// the layout sound; callers that save or send check this first.
    pub fn validate(&self) -> Result<(), DecodeError> {
        let m = &self.main;
        let long = |v: &Vec<u8>| v.len() > 0xff;
        if long(&m.name) || long(&m.description) || long(&m.address) {
            return Err(DecodeError::Invalid(
                "server settings string longer than 255 bytes",
            ));
        }
        for t in &self.trackers {
            if long(&t.name)
                || long(&t.comments)
                || long(&t.address)
                || long(&t.login)
                || long(&t.groups)
            {
                return Err(DecodeError::Invalid(
                    "tracker entry string longer than 255 bytes",
                ));
            }
        }
        for a in &self.allow_deny {
            if long(&a.address) || long(&a.name) || long(&a.comments) {
                return Err(DecodeError::Invalid(
                    "allow/deny entry string longer than 255 bytes",
                ));
            }
        }
        if self.trackers.len() > 0xff {
            return Err(DecodeError::Invalid("more than 255 trackers"));
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        self.encode_into(&mut w);
        w.into_inner()
    }

    /// The encoded bytes of section `i`, empty when absent.
    pub fn section_bytes(&self, i: usize) -> Vec<u8> {
        match i {
            Self::SECTION_MAIN => self.main.to_bytes(),
            Self::SECTION_TRACKERS => {
                let mut w = Writer::new();
                for t in &self.trackers {
                    t.encode(&mut w);
                }
                w.into_inner()
            }
            Self::SECTION_ALLOW_DENY => {
                let mut w = Writer::new();
                for a in &self.allow_deny {
                    a.encode(&mut w);
                }
                w.into_inner()
            }
            Self::SECTION_GREETING => match &self.greeting {
                Some(g) => {
                    let mut v = g.ignored.to_vec();
                    v.extend_from_slice(&g.text);
                    v
                }
                None => Vec::new(),
            },
            Self::SECTION_ICON => self.icon.clone().unwrap_or_default(),
            Self::SECTION_CHATS => self.chats.clone(),
            _ => self.other.get(i).cloned().unwrap_or_default(),
        }
    }

    /// The table as the encoder lays it out: sections in index order at
    /// 4-aligned offsets, an empty section at the running offset when it is a
    /// placeholder.
    pub fn table(&self) -> [(u32, u32); 11] {
        let mut table = [(0u32, 0u32); 11];
        let mut offset = Self::HEADER_LEN;
        for (i, entry) in table.iter_mut().enumerate() {
            let len = self.section_bytes(i).len();
            if len > 0 {
                *entry = (offset as u32, len as u32);
                offset += len + crate::wire::pad4_len(len);
            } else if self.placeholders[i] {
                *entry = (offset as u32, 0);
            }
        }
        table
    }

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        if r.array::<4>()? != *b"%KDX" || r.array::<4>()? != *b"SSTG" {
            return Err(DecodeError::Invalid("server settings magic"));
        }
        let version = r.u16()?;
        let ignored: [u8; 2] = r.array()?;
        let mut table = [(0u32, 0u32); 11];
        for entry in &mut table {
            *entry = (r.u32()?, r.u32()?);
        }
        let value_64: [u8; 11] = r.array()?;
        let tracker_count = r.u8()?;
        let mut placeholders = [false; 11];
        let mut sections: [Vec<u8>; 11] = Default::default();
        // Sections are read in table order; each must start where the
        // previous one (padded to 4) ended, which is what the originals write.
        for (i, &(offset, len)) in table.iter().enumerate() {
            placeholders[i] = offset != 0;
            if len == 0 {
                continue;
            }
            let here = r.pos() - start;
            let want = offset as usize;
            if want < here || want > here + 3 {
                return Err(DecodeError::Invalid("server settings section offset"));
            }
            let pad = r.bytes(want - here)?;
            if pad.iter().any(|&b| b != 0) {
                return Err(DecodeError::Invalid("server settings section padding"));
            }
            sections[i] = r.bytes(len as usize)?;
        }
        // The blob ends padded to 4 like every section before it.
        let tail = r.pos() - start;
        let pad = crate::wire::pad4_len(tail);
        if r.remaining() >= pad {
            let p = r.bytes(pad)?;
            if p.iter().any(|&b| b != 0) {
                return Err(DecodeError::Invalid("server settings tail padding"));
            }
        }
        let trailing = r.rest();
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sections;
        let main = ServerSettingsMain::decode(&s0)?;
        let trackers = decode_list(&s1, TrackerEntry::decode)?;
        if tracker_count as usize != trackers.len().min(255) {
            return Err(DecodeError::Invalid("server settings tracker count"));
        }
        let allow_deny = decode_list(&s2, AddressRule::decode)?;
        let greeting = if s3.is_empty() {
            None
        } else {
            let mut gr = Reader::new(&s3);
            Some(Greeting {
                ignored: gr.array()?,
                text: gr.rest(),
            })
        };
        let icon = (!s5.is_empty()).then_some(s5);
        let mut other: [Vec<u8>; 11] = Default::default();
        other[4] = s4;
        other[6] = s6;
        other[8] = s8;
        other[9] = s9;
        other[10] = s10;
        Ok(ServerSettings {
            version,
            ignored,
            value_64,
            placeholders,
            main,
            trackers,
            allow_deny,
            greeting,
            icon,
            chats: s7,
            other,
            trailing,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        let start = w.pos();
        w.bytes(b"%KDX");
        w.bytes(b"SSTG");
        w.u16(self.version);
        w.bytes(&self.ignored);
        for (offset, len) in self.table() {
            w.u32(offset);
            w.u32(len);
        }
        w.bytes(&self.value_64);
        w.u8(self.trackers.len().min(255) as u8);
        for i in 0..11 {
            let bytes = self.section_bytes(i);
            if bytes.is_empty() {
                continue;
            }
            w.pad4(start, &[]);
            w.bytes(&bytes);
        }
        w.pad4(start, &[]);
        w.bytes(&self.trailing);
    }
}

impl Default for ServerSettings {
    /// What a fresh Server 1.620 writes, minus the chat rooms.
    fn default() -> Self {
        ServerSettings {
            version: 1,
            ignored: [0; 2],
            value_64: [1, 1, 1, 1, 0, 1, 0, 2, 0, 0, 0],
            placeholders: Self::SERVER_PLACEHOLDERS,
            main: ServerSettingsMain::default(),
            trackers: Vec::new(),
            allow_deny: Vec::new(),
            greeting: None,
            icon: None,
            chats: Vec::new(),
            other: Default::default(),
            trailing: Vec::new(),
        }
    }
}

/// Zero padding of exactly the expected length is the norm and is kept as
/// the empty vector, so a value built by hand equals its decoded form;
/// anything else is kept as read.
fn canonical_padding(padding: Vec<u8>, expected: usize) -> Vec<u8> {
    if padding.len() == expected && padding.iter().all(|&b| b == 0) {
        Vec::new()
    } else {
        padding
    }
}

/// A string capped to what its one-byte length can say, so the length byte
/// and the data never disagree.
fn str255(v: &[u8]) -> &[u8] {
    &v[..v.len().min(255)]
}

/// Bytes between string-length bytes whose meaning is not pinned yet: a
/// non-zero value would mean strings this decoder does not know how to
/// skip, so it refuses rather than misread the rest.
fn unpinned_zero(bytes: &[u8]) -> Result<(), DecodeError> {
    if bytes.iter().any(|&b| b != 0) {
        return Err(DecodeError::Invalid(
            "server settings: unpinned length byte set",
        ));
    }
    Ok(())
}

fn decode_list<T>(
    bytes: &[u8],
    one: fn(&mut Reader) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    let mut r = Reader::new(bytes);
    let mut out = Vec::new();
    while !r.is_empty() {
        out.push(one(&mut r)?);
    }
    Ok(out)
}

/// Section 0. Offsets are from the section's start; the strings follow the
/// 0x5c-byte header in order and the section is padded to 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettingsMain {
    pub ignored_00: [u8; 18],
    /// 0 when not set.
    pub file_transfer_port: u16,
    pub ignored_14: [u8; 2],
    /// 0 when not set.
    pub alternate_port: u16,
    /// Server History → "Record Server History to File": bit 0 Login, 1
    /// Disconnected, 2 Download File, 3 Upload File, 4 Delete File, 5 Get
    /// File List, 6 Miscellaneous File, 7 Modify Account, 8 Miscellaneous
    /// Account, 9 Server Settings, 10 Miscellaneous (the window's order;
    /// bits 0 and 1 confirmed).
    pub history_kinds: u32,
    /// "Start new file when size exceeds", in bytes; 1 MiB by default.
    pub history_max_file_size: u32,
    pub ignored_20: [u8; 3],
    /// Bit 0: register with trackers. Bit 1: the greeting is on.
    pub flags: u8,
    pub ignored_24: [u8; 2],
    pub port: u16,
    pub ignored_28: [u8; 4],
    /// Twelve bytes that differ per server directory; probably an identifier.
    pub ignored_2c: [u8; 12],
    /// Bytes per second; 0 when off.
    pub total_outgoing_speed_limit: u32,
    pub ignored_3c: [u8; 22],
    /// Between the three length bytes (name at 0x52, description at 0x53,
    /// address at 0x55).
    pub ignored_54: u8,
    pub ignored_56: [u8; 6],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub address: Vec<u8>,
    /// Zero to three bytes, or whatever the file had after the strings.
    pub padding: Vec<u8>,
}

impl ServerSettingsMain {
    pub const HEADER_LEN: usize = 0x5c;

    pub fn greeting_enabled(&self) -> bool {
        self.flags & 2 != 0
    }

    pub fn register_with_trackers(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 18] = r.array()?;
        let file_transfer_port = r.u16()?;
        let ignored_14: [u8; 2] = r.array()?;
        let alternate_port = r.u16()?;
        let history_kinds = r.u32()?;
        let history_max_file_size = r.u32()?;
        let ignored_20: [u8; 3] = r.array()?;
        let flags = r.u8()?;
        let ignored_24: [u8; 2] = r.array()?;
        let port = r.u16()?;
        let ignored_28: [u8; 4] = r.array()?;
        let ignored_2c: [u8; 12] = r.array()?;
        let total_outgoing_speed_limit = r.u32()?;
        let ignored_3c: [u8; 22] = r.array()?;
        let name_len = r.u8()? as usize;
        let description_len = r.u8()? as usize;
        let ignored_54 = r.u8()?;
        unpinned_zero(&[ignored_54])?;
        let address_len = r.u8()? as usize;
        let ignored_56: [u8; 6] = r.array()?;
        let name = r.bytes(name_len)?;
        let description = r.bytes(description_len)?;
        let address = r.bytes(address_len)?;
        let expected = crate::wire::pad4_len(r.pos());
        let padding = canonical_padding(r.rest(), expected);
        Ok(ServerSettingsMain {
            ignored_00,
            file_transfer_port,
            ignored_14,
            alternate_port,
            history_kinds,
            history_max_file_size,
            ignored_20,
            flags,
            ignored_24,
            port,
            ignored_28,
            ignored_2c,
            total_outgoing_speed_limit,
            ignored_3c,
            ignored_54,
            ignored_56,
            name,
            description,
            address,
            padding,
        })
    }

    /// The length bytes come from the strings; `padding` is written as read
    /// when it is not the plain `pad4`, else recomputed.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.bytes(&self.ignored_00);
        w.u16(self.file_transfer_port);
        w.bytes(&self.ignored_14);
        w.u16(self.alternate_port);
        w.u32(self.history_kinds);
        w.u32(self.history_max_file_size);
        w.bytes(&self.ignored_20);
        w.u8(self.flags);
        w.bytes(&self.ignored_24);
        w.u16(self.port);
        w.bytes(&self.ignored_28);
        w.bytes(&self.ignored_2c);
        w.u32(self.total_outgoing_speed_limit);
        w.bytes(&self.ignored_3c);
        w.u8(str255(&self.name).len() as u8);
        w.u8(str255(&self.description).len() as u8);
        w.u8(self.ignored_54);
        w.u8(str255(&self.address).len() as u8);
        w.bytes(&self.ignored_56);
        w.bytes(str255(&self.name));
        w.bytes(str255(&self.description));
        w.bytes(str255(&self.address));
        if self.padding.is_empty() {
            w.pad4(0, &[]);
        } else {
            w.bytes(&self.padding);
        }
        w.into_inner()
    }
}

impl Default for ServerSettingsMain {
    fn default() -> Self {
        ServerSettingsMain {
            ignored_00: [0; 18],
            file_transfer_port: 0,
            ignored_14: [0; 2],
            alternate_port: 0,
            history_kinds: 0,
            history_max_file_size: 0x10_0000,
            ignored_20: [0; 3],
            flags: 0,
            ignored_24: [0; 2],
            port: 10700,
            ignored_28: [0; 4],
            ignored_2c: [0; 12],
            total_outgoing_speed_limit: 0,
            ignored_3c: [0; 22],
            ignored_54: 0,
            ignored_56: [0; 6],
            name: b"New Server".to_vec(),
            description: Vec::new(),
            address: Vec::new(),
            padding: Vec::new(),
        }
    }
}

/// One entry of section 1 (Server Settings → Trackers): a 0x20-byte header
/// with the string lengths at 0x14–0x1a, then the strings, padded to 4.
/// The tracker password typed into the window did not appear in the file;
/// where it goes is open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackerEntry {
    pub ignored_00: [u8; 20],
    pub ignored_18: [u8; 2],
    pub ignored_1b: u8,
    pub ignored_1c: [u8; 4],
    pub name: Vec<u8>,
    pub comments: Vec<u8>,
    pub address: Vec<u8>,
    pub login: Vec<u8>,
    pub groups: Vec<u8>,
    pub padding: Vec<u8>,
}

impl TrackerEntry {
    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let ignored_00: [u8; 20] = r.array()?;
        let name_len = r.u8()? as usize;
        let comments_len = r.u8()? as usize;
        let address_len = r.u8()? as usize;
        let login_len = r.u8()? as usize;
        let ignored_18: [u8; 2] = r.array()?;
        let groups_len = r.u8()? as usize;
        let ignored_1b = r.u8()?;
        unpinned_zero(&[ignored_18[0], ignored_18[1], ignored_1b])?;
        let ignored_1c: [u8; 4] = r.array()?;
        let name = r.bytes(name_len)?;
        let comments = r.bytes(comments_len)?;
        let address = r.bytes(address_len)?;
        let login = r.bytes(login_len)?;
        let groups = r.bytes(groups_len)?;
        let pad = r.pad4(start)?;
        let padding = canonical_padding(pad.clone(), pad.len());
        Ok(TrackerEntry {
            ignored_00,
            ignored_18,
            ignored_1b,
            ignored_1c,
            name,
            comments,
            address,
            login,
            groups,
            padding,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        w.bytes(&self.ignored_00);
        w.u8(str255(&self.name).len() as u8);
        w.u8(str255(&self.comments).len() as u8);
        w.u8(str255(&self.address).len() as u8);
        w.u8(str255(&self.login).len() as u8);
        w.bytes(&self.ignored_18);
        w.u8(str255(&self.groups).len() as u8);
        w.u8(self.ignored_1b);
        w.bytes(&self.ignored_1c);
        w.bytes(str255(&self.name));
        w.bytes(str255(&self.comments));
        w.bytes(str255(&self.address));
        w.bytes(str255(&self.login));
        w.bytes(str255(&self.groups));
        w.pad4(start, &self.padding);
    }
}

/// One entry of section 2 (Server Settings → Allow/Deny Addresses): a
/// 0x1c-byte header, then address, name and comments, padded to 4. The
/// server drops entries whose expiry has passed when it next saves.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddressRule {
    /// 0 when the rule does not expire.
    pub expires: Time1900,
    pub ignored_08: [u8; 8],
    /// Allow when true, deny when false (`00 01` / `00 00` at 0x10).
    pub allow: bool,
    pub ignored_12: [u8; 6],
    pub ignored_1b: u8,
    pub address: Vec<u8>,
    pub name: Vec<u8>,
    pub comments: Vec<u8>,
    pub padding: Vec<u8>,
}

impl AddressRule {
    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let expires = Time1900::decode(r)?;
        let ignored_08: [u8; 8] = r.array()?;
        let allow = match r.u16()? {
            0 => false,
            1 => true,
            _ => return Err(DecodeError::Invalid("allow/deny flag")),
        };
        let ignored_12: [u8; 6] = r.array()?;
        let address_len = r.u8()? as usize;
        let name_len = r.u8()? as usize;
        let comments_len = r.u8()? as usize;
        let ignored_1b = r.u8()?;
        unpinned_zero(&[ignored_1b])?;
        let address = r.bytes(address_len)?;
        let name = r.bytes(name_len)?;
        let comments = r.bytes(comments_len)?;
        let pad = r.pad4(start)?;
        let padding = canonical_padding(pad.clone(), pad.len());
        Ok(AddressRule {
            expires,
            ignored_08,
            allow,
            ignored_12,
            ignored_1b,
            address,
            name,
            comments,
            padding,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        self.expires.encode(w);
        w.bytes(&self.ignored_08);
        w.u16(self.allow as u16);
        w.bytes(&self.ignored_12);
        w.u8(str255(&self.address).len() as u8);
        w.u8(str255(&self.name).len() as u8);
        w.u8(str255(&self.comments).len() as u8);
        w.u8(self.ignored_1b);
        w.bytes(str255(&self.address));
        w.bytes(str255(&self.name));
        w.bytes(str255(&self.comments));
        w.pad4(start, &self.padding);
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

impl Default for HistoryRecord {
    fn default() -> Self {
        HistoryRecord {
            time: Time1900::default(),
            kind: HistoryKind::default(),
            user_id: 0,
            ignored_10: [0; 50],
            ignored_44: [0; 2],
            ignored_4a: [0; 2],
            address: Vec::new(),
            nick: Vec::new(),
            login: Vec::new(),
            detail: Vec::new(),
            path: FilePath::default(),
            padding: Vec::new(),
        }
    }
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
