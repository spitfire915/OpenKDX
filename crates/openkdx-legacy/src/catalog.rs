//! `Catalog.dat`: the file catalog Search (452) reads, written by Generate
//! Catalog (450). Clear text, `%FlK`. See `docs/formats/catalog.md`.
//!
//! A header, one entry per folder and file in depth-first order (the entry
//! holds its parent's index), then an index: a 32-bit letter mask per entry
//! and the entries' offsets.

use openkdx_proto::msg::common::Time1900;
use openkdx_proto::wire::{DecodeError, Reader, Writer};

use crate::error::{LegacyError, Result};
use crate::store::LegacyDir;

pub const CATALOG_FILE: &str = "Catalog.dat";
pub const MAGIC: [u8; 4] = *b"%FlK";
const HEADER_LEN: usize = 0x30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub version: u16,
    pub ignored_06: u16,
    pub generated: Time1900,
    pub ignored_18: [u8; 24],
    pub entries: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogEntry {
    /// 0 for a folder.
    pub size: u64,
    pub created: Time1900,
    pub modified: Time1900,
    /// 1 file, 2 folder.
    pub kind: u32,
    /// Folders: every item below, at any depth. Files: 0.
    pub descendants: u32,
    /// Index of the parent entry; the root `Bases` says 0.
    pub parent: u32,
    pub ignored_24: u32,
    /// `0x03` folder, `0x10` file.
    pub flags: u8,
    pub ignored_29: u8,
    pub ignored_2b: u8,
    pub name: Vec<u8>,
    /// Non-zero `pad4` bytes as read, else empty.
    pub padding: Vec<u8>,
    /// The search mask from the index; `name_mask` computes it.
    pub mask: u32,
}

impl CatalogEntry {
    pub const KIND_FILE: u32 = 1;
    pub const KIND_FOLDER: u32 = 2;

    pub fn is_file(&self) -> bool {
        self.kind == Self::KIND_FILE
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Catalog {
            version: 1,
            ignored_06: 1,
            generated: Time1900(0),
            ignored_18: [0; 24],
            entries: Vec::new(),
        }
    }
}

impl Catalog {
    /// Appends an entry, computing its mask when it is zero.
    pub fn push(&mut self, mut e: CatalogEntry) {
        if e.mask == 0 {
            e.mask = name_mask(&e.name);
        }
        self.entries.push(e);
    }

    /// The names from the root down to entry `i`.
    pub fn path_of(&self, i: usize) -> Vec<&[u8]> {
        let mut out = Vec::new();
        let mut at = i;
        loop {
            let Some(e) = self.entries.get(at) else { break };
            out.push(e.name.as_slice());
            if at == 0 || e.parent as usize == at {
                break;
            }
            at = e.parent as usize;
        }
        out.reverse();
        out
    }
}

/// One bit per letter of the name, case-insensitively: `a` = bit 0 … `z` =
/// bit 25; a space sets bit 27, `.` bit 28, `[` and `]` bit 31. Other
/// characters are not pinned yet and set bit 31 `[I]`.
pub fn name_mask(name: &[u8]) -> u32 {
    let mut m = 0u32;
    for &b in name {
        let bit = match b.to_ascii_lowercase() {
            c @ b'a'..=b'z' => c - b'a',
            b' ' => 27,
            b'.' => 28,
            _ => 31,
        };
        m |= 1 << bit;
    }
    m
}

pub fn decode_catalog(bytes: &[u8]) -> Result<Catalog> {
    if bytes.len() < 4 {
        return Err(LegacyError::Decode(DecodeError::Truncated {
            offset: bytes.len(),
            needed: 4 - bytes.len(),
        }));
    }
    if bytes[..4] != MAGIC {
        let mut found = [0u8; 8];
        for (i, b) in bytes.iter().take(8).enumerate() {
            found[i] = *b;
        }
        let mut expected = [0u8; 8];
        expected[..4].copy_from_slice(&MAGIC);
        return Err(LegacyError::BadMagic { expected, found });
    }
    let mut r = Reader::new(bytes);
    r.bytes(4)?;
    let version = r.u16()?;
    if version != 1 {
        return Err(LegacyError::UnsupportedVersion {
            file: CATALOG_FILE,
            version,
        });
    }
    let ignored_06 = r.u16()?;
    let count = r.u32()? as usize;
    let index_offset = r.u32()? as usize;
    let generated = Time1900::decode(&mut r)?;
    let ignored_18: [u8; 24] = r.array()?;
    debug_assert_eq!(r.pos(), HEADER_LEN);
    let mut entries = Vec::with_capacity(count);
    let mut offsets = Vec::with_capacity(count);
    for _ in 0..count {
        let start = r.pos();
        offsets.push(start as u32);
        let size = r.u64()?;
        let created = Time1900::decode(&mut r)?;
        let modified = Time1900::decode(&mut r)?;
        let kind = r.u32()?;
        let descendants = r.u32()?;
        let parent = r.u32()?;
        let ignored_24 = r.u32()?;
        let flags = r.u8()?;
        let ignored_29 = r.u8()?;
        let name_len = r.u8()? as usize;
        let ignored_2b = r.u8()?;
        let name = r.bytes(name_len)?;
        let mut padding = r.pad4(start)?;
        if padding.iter().all(|&b| b == 0) {
            padding.clear();
        }
        entries.push(CatalogEntry {
            size,
            created,
            modified,
            kind,
            descendants,
            parent,
            ignored_24,
            flags,
            ignored_29,
            ignored_2b,
            name,
            padding,
            mask: 0,
        });
    }
    if r.pos() != index_offset {
        return Err(LegacyError::Invalid("Catalog.dat: index offset"));
    }
    for e in &mut entries {
        e.mask = r.u32()?;
    }
    for (i, want) in offsets.iter().enumerate() {
        if r.u32()? != *want {
            let _ = i;
            return Err(LegacyError::Invalid("Catalog.dat: entry offset table"));
        }
    }
    r.finish()?;
    Ok(Catalog {
        version,
        ignored_06,
        generated,
        ignored_18,
        entries,
    })
}

pub fn encode_catalog(c: &Catalog) -> Vec<u8> {
    let mut body = Writer::new();
    let mut offsets = Vec::with_capacity(c.entries.len());
    for e in &c.entries {
        let start = HEADER_LEN + body.pos();
        offsets.push(start as u32);
        body.u64(e.size);
        e.created.encode(&mut body);
        e.modified.encode(&mut body);
        body.u32(e.kind);
        body.u32(e.descendants);
        body.u32(e.parent);
        body.u32(e.ignored_24);
        body.u8(e.flags);
        body.u8(e.ignored_29);
        body.u8(e.name.len().min(255) as u8);
        body.u8(e.ignored_2b);
        body.bytes(&e.name[..e.name.len().min(255)]);
        body.pad4(start - HEADER_LEN, &e.padding);
    }
    let index_offset = HEADER_LEN + body.pos();
    let mut w = Writer::new();
    w.bytes(&MAGIC);
    w.u16(c.version);
    w.u16(c.ignored_06);
    w.u32(c.entries.len() as u32);
    w.u32(index_offset as u32);
    c.generated.encode(&mut w);
    w.bytes(&c.ignored_18);
    w.bytes(&body.into_inner());
    for e in &c.entries {
        w.u32(e.mask);
    }
    for o in offsets {
        w.u32(o);
    }
    w.into_inner()
}

impl LegacyDir {
    pub fn catalog_path(&self) -> std::path::PathBuf {
        self.root().join(CATALOG_FILE)
    }

    pub fn load_catalog(&self) -> Result<Option<Catalog>> {
        let path = self.catalog_path();
        match std::fs::read(&path) {
            Ok(b) => Ok(Some(decode_catalog(&b)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(LegacyError::io(path, e)),
        }
    }

    /// The catalog is regenerated, never edited in place, so it is written
    /// without a backup: temp file and rename only.
    pub fn save_catalog(&self, c: &Catalog) -> Result<()> {
        let path = self.catalog_path();
        let tmp = self.root().join(format!("{CATALOG_FILE}.tmp"));
        std::fs::write(&tmp, encode_catalog(c)).map_err(|e| LegacyError::io(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| LegacyError::io(&path, e))
    }
}
