//! `KDX.stg`: the client's settings, scrambled with [`Stream::CLIENT_SETTINGS`].
//! See `docs/formats/client-stg.md`.
//!
//! Plain layout: `%KDXSTNG`, version 2, a table of 12 `(offset, length)`
//! sections, 16 bytes, then the sections, contiguous, in the order the
//! client wrote them (not index order). Section 1 is the identity, section
//! 7 the address book; the rest (window positions, settings, registration)
//! are kept as bytes for Phase 7.

use openkdx_proto::msg::common::Color;
use openkdx_proto::wire::{Reader, Writer};

use crate::error::{LegacyError, Result};
use crate::scramble::{Stream, scramble, unscramble};
use crate::server_stg::check_header;

pub const MAGIC: [u8; 8] = *b"%KDXSTNG";
pub const FILE: &str = "KDX.stg";
const HEADER_LEN: usize = 0x80;
const SECTION_IDENTITY: usize = 1;
const SECTION_ADDRESS_BOOK: usize = 7;
const LIST_MAGIC: u32 = 0x38fd_db01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSettings {
    pub version: u16,
    pub ignored_0a: [u8; 2],
    pub ignored_0c: [u8; 4],
    /// `00 00 00 00 03 01 00 00 00 00 00 02 01 01 01 01` in every file.
    pub ignored_70: [u8; 16],
    /// Section indices in the order they sit in the file.
    pub order: Vec<usize>,
    pub identity: ClientIdentity,
    pub address_book: Vec<AddressBookEntry>,
    /// The address-book list's second word (3 with three entries; 1 with
    /// one): kept as read, rewritten as the entry count when zero.
    pub address_book_count2: u32,
    /// Every other section's bytes, by index (1 and 7 stay empty here).
    pub sections: [Vec<u8>; 12],
    /// Bytes after the last section.
    pub trailing: Vec<u8>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        ClientSettings {
            version: 2,
            ignored_0a: [0; 2],
            ignored_0c: [0; 4],
            ignored_70: [0, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 2, 1, 1, 1, 1],
            order: vec![0, 1, 7, 8, 10, 9, 11],
            identity: ClientIdentity::default(),
            address_book: Vec::new(),
            address_book_count2: 0,
            sections: Default::default(),
            trailing: Vec::new(),
        }
    }
}

/// Section 1: name, description, colours and the 32×32 icon at 4 bits per
/// pixel (512 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    pub ignored_00: [u8; 12],
    pub ignored_0c: [u8; 4],
    pub ignored_10: [u8; 12],
    pub ignored_1e: [u8; 2],
    pub ignored_20: [u8; 2],
    pub ignored_22: [u8; 3],
    pub background_color: Color,
    pub text_color: Color,
    pub icon: Vec<u8>,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

impl Default for ClientIdentity {
    fn default() -> Self {
        ClientIdentity {
            ignored_00: [0; 12],
            ignored_0c: [0, 0xff, 0xcc, 0],
            ignored_10: [0; 12],
            ignored_1e: [0; 2],
            ignored_20: [1, 2],
            ignored_22: [0xff; 3],
            background_color: Color {
                r: 0xff,
                g: 0xcc,
                b: 0,
            },
            text_color: Color { r: 0, g: 0, b: 0 },
            icon: vec![0; 512],
            name: Vec::new(),
            description: Vec::new(),
            padding: Vec::new(),
        }
    }
}

/// One address-book record: a 0x84-byte header (flags, the password as the
/// 32-byte derived key at 0x1c, string lengths at 0x7c–0x80), then name,
/// description, comments, address, login, padded to 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBookEntry {
    /// From the list header; 0 means "assign on encode".
    pub id: u32,
    pub ignored_00: [u8; 16],
    /// `01 00 00 00` on the stock entry, `00 00 00 80` on one made in the
    /// window; the dialog's check boxes, not pinned.
    pub flags: u32,
    pub ignored_14: u32,
    pub ignored_18: u32,
    /// `AccountKey::from_password(password)`; the guest key when empty.
    pub key: [u8; 32],
    pub ignored_3c: [u8; 0x40],
    pub ignored_81: [u8; 3],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub comments: Vec<u8>,
    pub address: Vec<u8>,
    pub login: Vec<u8>,
    pub padding: Vec<u8>,
}

impl ClientIdentity {
    const HEADER_LEN: usize = 0x2b;
    const ICON_LEN: usize = 512;

    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 12] = r.array()?;
        let ignored_0c: [u8; 4] = r.array()?;
        let ignored_10: [u8; 12] = r.array()?;
        let name_len = r.u8()? as usize;
        let description_len = r.u8()? as usize;
        let ignored_1e: [u8; 2] = r.array()?;
        let ignored_20: [u8; 2] = r.array()?;
        let ignored_22: [u8; 3] = r.array()?;
        let background_color = Color::decode(&mut r)?;
        let text_color = Color::decode(&mut r)?;
        debug_assert_eq!(r.pos(), Self::HEADER_LEN);
        let icon = r.bytes(Self::ICON_LEN)?;
        let name = r.bytes(name_len)?;
        let description = r.bytes(description_len)?;
        let mut padding = r.rest();
        if padding.len() == (4 - r.pos() % 4) % 4 && padding.iter().all(|&b| b == 0) {
            padding.clear();
        }
        Ok(ClientIdentity {
            ignored_00,
            ignored_0c,
            ignored_10,
            ignored_1e,
            ignored_20,
            ignored_22,
            background_color,
            text_color,
            icon,
            name,
            description,
            padding,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.bytes(&self.ignored_00);
        w.bytes(&self.ignored_0c);
        w.bytes(&self.ignored_10);
        w.u8(self.name.len().min(255) as u8);
        w.u8(self.description.len().min(255) as u8);
        w.bytes(&self.ignored_1e);
        w.bytes(&self.ignored_20);
        w.bytes(&self.ignored_22);
        self.background_color.encode(&mut w);
        self.text_color.encode(&mut w);
        let mut icon = self.icon.clone();
        icon.resize(Self::ICON_LEN, 0);
        w.bytes(&icon);
        w.bytes(&self.name[..self.name.len().min(255)]);
        w.bytes(&self.description[..self.description.len().min(255)]);
        if self.padding.is_empty() {
            w.pad4(0, &[]);
        } else {
            w.bytes(&self.padding);
        }
        w.into_inner()
    }
}

impl Default for AddressBookEntry {
    fn default() -> Self {
        AddressBookEntry {
            id: 0,
            ignored_00: [0; 16],
            flags: 0x80,
            ignored_14: 0,
            ignored_18: 0,
            key: *openkdx_proto::AccountKey::GUEST.as_bytes(),
            ignored_3c: [0; 0x40],
            ignored_81: [0; 3],
            name: Vec::new(),
            description: Vec::new(),
            comments: Vec::new(),
            address: Vec::new(),
            login: Vec::new(),
            padding: Vec::new(),
        }
    }
}

impl AddressBookEntry {
    fn decode(r: &mut Reader) -> Result<Self> {
        let start = r.pos();
        let ignored_00: [u8; 16] = r.array()?;
        let flags = r.u32()?;
        let ignored_14 = r.u32()?;
        let ignored_18 = r.u32()?;
        let key: [u8; 32] = r.array()?;
        let ignored_3c: [u8; 0x40] = r.array()?;
        let name_len = r.u8()? as usize;
        let description_len = r.u8()? as usize;
        let comments_len = r.u8()? as usize;
        let address_len = r.u8()? as usize;
        let login_len = r.u8()? as usize;
        let ignored_81: [u8; 3] = r.array()?;
        let name = r.bytes(name_len)?;
        let description = r.bytes(description_len)?;
        let comments = r.bytes(comments_len)?;
        let address = r.bytes(address_len)?;
        let login = r.bytes(login_len)?;
        let mut padding = r.pad4(start)?;
        if padding.iter().all(|&b| b == 0) {
            padding.clear();
        }
        Ok(AddressBookEntry {
            id: 0,
            ignored_00,
            flags,
            ignored_14,
            ignored_18,
            key,
            ignored_3c,
            ignored_81,
            name,
            description,
            comments,
            address,
            login,
            padding,
        })
    }

    fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        w.bytes(&self.ignored_00);
        w.u32(self.flags);
        w.u32(self.ignored_14);
        w.u32(self.ignored_18);
        w.bytes(&self.key);
        w.bytes(&self.ignored_3c);
        let cap = |v: &Vec<u8>| v.len().min(255);
        w.u8(cap(&self.name) as u8);
        w.u8(cap(&self.description) as u8);
        w.u8(cap(&self.comments) as u8);
        w.u8(cap(&self.address) as u8);
        w.u8(cap(&self.login) as u8);
        w.bytes(&self.ignored_81);
        for s in [
            &self.name,
            &self.description,
            &self.comments,
            &self.address,
            &self.login,
        ] {
            w.bytes(&s[..cap(s)]);
        }
        w.pad4(start, &self.padding);
    }
}

fn decode_address_book(bytes: &[u8]) -> Result<(u32, Vec<AddressBookEntry>)> {
    if bytes.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut r = Reader::new(bytes);
    if r.u32()? != LIST_MAGIC {
        return Err(LegacyError::Invalid("KDX.stg: address book magic"));
    }
    let count = r.u32()? as usize;
    let count2 = r.u32()?;
    if r.u32()? != 0 {
        return Err(LegacyError::Invalid("KDX.stg: address book header"));
    }
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(r.u32()?);
    }
    if r.u32()? != 0 {
        return Err(LegacyError::Invalid("KDX.stg: address book header"));
    }
    let mut ends = Vec::with_capacity(count);
    for _ in 0..count {
        ends.push(r.u32()? as usize);
    }
    let records = r.rest();
    let mut out = Vec::with_capacity(count);
    let mut start = 0;
    for (id, end) in ids.into_iter().zip(ends) {
        let slice = records.get(start..end).ok_or(LegacyError::Invalid(
            "KDX.stg: address book record past its list",
        ))?;
        let mut rr = Reader::new(slice);
        let mut e = AddressBookEntry::decode(&mut rr)?;
        if !rr.is_empty() {
            return Err(LegacyError::Invalid("KDX.stg: address book record"));
        }
        e.id = id;
        out.push(e);
        start = end;
    }
    if start != records.len() {
        return Err(LegacyError::Invalid(
            "KDX.stg: bytes after the address book",
        ));
    }
    Ok((count2, out))
}

fn encode_address_book(count2: u32, entries: &[AddressBookEntry]) -> Vec<u8> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut records = Writer::new();
    let mut ends = Vec::with_capacity(entries.len());
    for e in entries {
        e.encode(&mut records);
        ends.push(records.pos() as u32);
    }
    let mut w = Writer::new();
    w.u32(LIST_MAGIC);
    w.u32(entries.len() as u32);
    w.u32(if count2 == 0 {
        entries.len() as u32
    } else {
        count2
    });
    w.u32(0);
    let mut next = 1;
    for e in entries {
        let id = if e.id == 0 { next } else { e.id };
        next = id + 1;
        w.u32(id);
    }
    w.u32(0);
    for end in ends {
        w.u32(end);
    }
    w.bytes(&records.into_inner());
    w.into_inner()
}

pub fn decode_client_stg(bytes: &[u8]) -> Result<ClientSettings> {
    let plain = unscramble(bytes, Stream::CLIENT_SETTINGS);
    check_header(&plain, MAGIC, 2, FILE)?;
    let mut r = Reader::new(&plain);
    r.bytes(8)?;
    let version = r.u16()?;
    let ignored_0a: [u8; 2] = r.array()?;
    let ignored_0c: [u8; 4] = r.array()?;
    let mut table = [(0usize, 0usize); 12];
    for entry in &mut table {
        *entry = (r.u32()? as usize, r.u32()? as usize);
    }
    let ignored_70: [u8; 16] = r.array()?;
    debug_assert_eq!(r.pos(), HEADER_LEN);
    let mut order: Vec<usize> = (0..12).filter(|&i| table[i].1 > 0).collect();
    order.sort_by_key(|&i| table[i].0);
    let mut sections: [Vec<u8>; 12] = Default::default();
    for &i in &order {
        let (offset, len) = table[i];
        if offset != r.pos() {
            return Err(LegacyError::Invalid("KDX.stg: sections are not contiguous"));
        }
        sections[i] = r.bytes(len)?;
    }
    let trailing = r.rest();
    let identity = ClientIdentity::decode(&std::mem::take(&mut sections[SECTION_IDENTITY]))?;
    let (count2, address_book) =
        decode_address_book(&std::mem::take(&mut sections[SECTION_ADDRESS_BOOK]))?;
    Ok(ClientSettings {
        version,
        ignored_0a,
        ignored_0c,
        ignored_70,
        order,
        identity,
        address_book,
        address_book_count2: count2,
        sections,
        trailing,
    })
}

pub fn encode_client_stg(s: &ClientSettings) -> Vec<u8> {
    let mut sections = s.sections.clone();
    sections[SECTION_IDENTITY] = s.identity.encode();
    sections[SECTION_ADDRESS_BOOK] = encode_address_book(s.address_book_count2, &s.address_book);
    // Sections the order list does not name (new ones) go last, by index.
    let mut order = s.order.clone();
    for (i, sec) in sections.iter().enumerate() {
        if !sec.is_empty() && !order.contains(&i) {
            order.push(i);
        }
    }
    let mut table = [(0u32, 0u32); 12];
    let mut body = Writer::new();
    for &i in &order {
        if sections[i].is_empty() {
            continue;
        }
        table[i] = ((HEADER_LEN + body.pos()) as u32, sections[i].len() as u32);
        body.bytes(&sections[i]);
    }
    let mut w = Writer::new();
    w.bytes(&MAGIC);
    w.u16(s.version);
    w.bytes(&s.ignored_0a);
    w.bytes(&s.ignored_0c);
    for (o, l) in table {
        w.u32(o);
        w.u32(l);
    }
    w.bytes(&s.ignored_70);
    w.bytes(&body.into_inner());
    w.bytes(&s.trailing);
    scramble(&w.into_inner(), Stream::CLIENT_SETTINGS)
}
