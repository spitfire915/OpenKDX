//! Messages 700–721: the tracker's TCP side. See the reference spec's
//! `messages/tracker.md`. The UDP registration is in [`crate::registration`].

use crate::message_type::MessageType;
use crate::msg::common::Time1900;
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

empty_payload! {
    /// 700: reply is 701.
    GetTrackerGroupList = GET_TRACKER_GROUP_LIST;
    /// 710: reply is 711.
    GetTrackerSettings = GET_TRACKER_SETTINGS;
    /// 713: reply to 712.
    SetTrackerSettingsDone = SET_TRACKER_SETTINGS_DONE;
    /// 720: reply is 721.
    GetTrackerStatistics = GET_TRACKER_STATISTICS;
}

/// One group in 701.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupListEntry {
    /// Servers in the group.
    pub servers: u16,
    /// Not set by the tracker, ignored by the client.
    pub value_02: [u8; 4],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 701: reply to 700.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackerGroupList {
    pub groups: Vec<GroupListEntry>,
}

impl Payload for TrackerGroupList {
    const KIND: MessageType = MessageType::TRACKER_GROUP_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut groups = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            let start = r.pos();
            let servers = r.u16()?;
            let value_02: [u8; 4] = r.array()?;
            let name_len = usize::from(r.u8()?);
            let description_len = usize::from(r.u8()?);
            groups.push(GroupListEntry {
                servers,
                value_02,
                name: r.bytes(name_len)?,
                description: r.bytes(description_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(TrackerGroupList { groups })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.groups.len() as u32);
        for g in &self.groups {
            let start = w.pos();
            w.u16(g.servers);
            w.bytes(&g.value_02);
            w.u8(g.name.len() as u8);
            w.u8(g.description.len() as u8);
            w.bytes(&g.name);
            w.bytes(&g.description);
            w.pad4(start, &g.padding);
        }
    }
}

/// 702: reply is 703. The group name must match byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GetServerList {
    pub ignored_00: [u8; 2],
    pub ignored_03: u8,
    pub group: Vec<u8>,
}

impl Payload for GetServerList {
    const KIND: MessageType = MessageType::GET_TRACKER_SERVER_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 2] = r.array()?;
        let group_len = usize::from(r.u8()?);
        let ignored_03 = r.u8()?;
        let group = r.bytes(group_len)?;
        r.finish()?;
        Ok(GetServerList {
            ignored_00,
            ignored_03,
            group,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored_00);
        w.u8(self.group.len() as u8);
        w.u8(self.ignored_03);
        w.bytes(&self.group);
    }
}

/// One server in 703.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerListEntry {
    /// When the tracker first saw the registration.
    pub first_seen: Time1900,
    pub ignored_08: [u8; 6],
    /// From the registration.
    pub flags: u16,
    pub users: u16,
    pub ignored_12: [u8; 6],
    /// IPv4, such as 0x7f000001.
    pub address: u32,
    pub port: u16,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    /// Length 0 as seen.
    pub extra: Vec<u8>,
    pub padding: Vec<u8>,
}

impl ServerListEntry {
    pub fn socket_addr(&self) -> std::net::SocketAddrV4 {
        std::net::SocketAddrV4::new(self.address.into(), self.port)
    }
}

/// 703: reply to 702.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackerServerList {
    pub servers: Vec<ServerListEntry>,
}

impl Payload for TrackerServerList {
    const KIND: MessageType = MessageType::TRACKER_SERVER_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut servers = Vec::with_capacity(count.min(0xffff));
        for _ in 0..count {
            let start = r.pos();
            let first_seen = Time1900::decode(&mut r)?;
            let ignored_08: [u8; 6] = r.array()?;
            let flags = r.u16()?;
            let users = r.u16()?;
            let ignored_12: [u8; 6] = r.array()?;
            let address_len = usize::from(r.u8()?);
            let name_len = usize::from(r.u8()?);
            let description_len = usize::from(r.u8()?);
            let extra_len = usize::from(r.u8()?);
            if address_len != 6 {
                return Err(DecodeError::Invalid("server list address is not 6 bytes"));
            }
            let address = r.u32()?;
            let port = r.u16()?;
            servers.push(ServerListEntry {
                first_seen,
                ignored_08,
                flags,
                users,
                ignored_12,
                address,
                port,
                name: r.bytes(name_len)?,
                description: r.bytes(description_len)?,
                extra: r.bytes(extra_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(TrackerServerList { servers })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.servers.len() as u32);
        for s in &self.servers {
            let start = w.pos();
            s.first_seen.encode(w);
            w.bytes(&s.ignored_08);
            w.u16(s.flags);
            w.u16(s.users);
            w.bytes(&s.ignored_12);
            w.u8(6);
            w.u8(s.name.len() as u8);
            w.u8(s.description.len() as u8);
            w.u8(s.extra.len() as u8);
            w.u32(s.address);
            w.u16(s.port);
            w.bytes(&s.name);
            w.bytes(&s.description);
            w.bytes(&s.extra);
            w.pad4(start, &s.padding);
        }
    }
}

/// The tracker's settings blob, as 711 and 712 carry it: `%KDX` `TSTG`, a
/// version, a table of 12 sections, and the sections' bytes.
///
/// Section 0 is general settings, 1 upstream trackers, 2 groups, 4 the
/// greeting, 5 admin; the rest are unused. The sections' own layouts are
/// kept as bytes here; the tracker (Phase 5) parses them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerSettings {
    /// Must be 1.
    pub version: u16,
    pub ignored: [u8; 2],
    /// `None` for an absent section (offset 0, size 0, present byte 0).
    pub sections: [Option<Vec<u8>>; 12],
}

impl TrackerSettings {
    pub const HEADER_LEN: usize = 0x78;
    pub const SECTION_GENERAL: usize = 0;
    pub const SECTION_UPSTREAM: usize = 1;
    pub const SECTION_GROUPS: usize = 2;
    pub const SECTION_GREETING: usize = 4;
    pub const SECTION_ADMIN: usize = 5;

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let all = r.rest();
        if all.len() < Self::HEADER_LEN {
            return Err(DecodeError::Truncated {
                offset: all.len(),
                needed: Self::HEADER_LEN - all.len(),
            });
        }
        let mut h = Reader::new(&all);
        if h.array::<4>()? != *b"%KDX" || h.array::<4>()? != *b"TSTG" {
            return Err(DecodeError::Invalid("tracker settings magic"));
        }
        let version = h.u16()?;
        if version != 1 {
            return Err(DecodeError::Invalid("tracker settings version is not 1"));
        }
        let ignored: [u8; 2] = h.array()?;
        let mut table = [(0u32, 0u32); 12];
        for entry in &mut table {
            *entry = (h.u32()?, h.u32()?);
        }
        let present: [u8; 12] = h.array()?;
        let mut sections: [Option<Vec<u8>>; 12] = Default::default();
        let mut end = Self::HEADER_LEN;
        for (i, (offset, size)) in table.iter().enumerate() {
            if present[i] != 1 {
                continue;
            }
            let (offset, size) = (*offset as usize, *size as usize);
            let data = all
                .get(offset..offset.saturating_add(size))
                .ok_or(DecodeError::Invalid(
                    "tracker settings section out of range",
                ))?;
            sections[i] = Some(data.to_vec());
            end = end.max(offset + size);
        }
        if end != all.len() {
            return Err(DecodeError::Trailing(all.len() - end));
        }
        Ok(TrackerSettings {
            version,
            ignored,
            sections,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        w.bytes(b"%KDX");
        w.bytes(b"TSTG");
        w.u16(self.version);
        w.bytes(&self.ignored);
        let mut offset = Self::HEADER_LEN;
        for s in &self.sections {
            match s {
                Some(data) => {
                    w.u32(offset as u32);
                    w.u32(data.len() as u32);
                    offset += data.len();
                }
                None => {
                    w.u32(0);
                    w.u32(0);
                }
            }
        }
        for s in &self.sections {
            w.u8(u8::from(s.is_some()));
        }
        for data in self.sections.iter().flatten() {
            w.bytes(data);
        }
    }
}

macro_rules! tracker_settings_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name {
                pub settings: TrackerSettings,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let settings = TrackerSettings::decode_from(&mut r)?;
                    Ok($name { settings })
                }

                fn encode(&self, w: &mut Writer) {
                    self.settings.encode_into(w);
                }
            }
        )*
    };
}

tracker_settings_payload! {
    /// 711: reply to 710.
    TrackerSettingsInfo = TRACKER_SETTINGS_INFO;
    /// 712: reply is 713.
    SetTrackerSettings = SET_TRACKER_SETTINGS;
}

/// 721: reply to 720.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrackerStats {
    pub start_time: Time1900,
    pub connections_accepted: u64,
    pub datagrams_received: u64,
    pub ignored_18: [u8; 8],
    pub hours: u32,
    /// TCP connections open now.
    pub connections: u32,
    /// Records listed.
    pub servers: u32,
    pub ignored_2c: [u8; 12],
}

impl Payload for TrackerStats {
    const KIND: MessageType = MessageType::TRACKER_STATISTICS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = TrackerStats {
            start_time: Time1900::decode(&mut r)?,
            connections_accepted: r.u64()?,
            datagrams_received: r.u64()?,
            ignored_18: r.array()?,
            hours: r.u32()?,
            connections: r.u32()?,
            servers: r.u32()?,
            ignored_2c: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        self.start_time.encode(w);
        w.u64(self.connections_accepted);
        w.u64(self.datagrams_received);
        w.bytes(&self.ignored_18);
        w.u32(self.hours);
        w.u32(self.connections);
        w.u32(self.servers);
        w.bytes(&self.ignored_2c);
    }
}
