//! `.hap` appearance files, as the KDX Client loads them and Appearance
//! Editor 1.240 writes them. See `docs/formats/hap.md`.
//!
//! `%HAP`, version 1, a table of 13 `(offset, length)` sections, 20 bytes,
//! then the sections at 4-aligned offsets. Section 4 is the information
//! page, 5 the images, 6 the 204 colours, 7 the icons; the rest have never
//! been seen non-empty. Images and icons are slot tables (528 and 768
//! `u32` offsets, one per UI element, 0 for "none", several slots may share
//! a record) followed by bitmap records: width, height, a flag, the depth
//! in bits (1, 2, 4, 8), the palette size minus one, a byte, for images
//! twelve more bytes, the palette as `00 RR GG BB`, then rows of packed
//! pixels (most significant bits first) padded to 4 bytes.

use openkdx_proto::msg::common::Time1900;
use openkdx_proto::wire::{DecodeError, Reader, Writer, pad4_len};

use crate::error::{LegacyError, Result};

pub const MAGIC: [u8; 8] = *b"%HAP\x00\x01\x00\x00";
const TABLE_LEN: usize = 13;
const HEADER_LEN: usize = 0x88;
const SECTION_INFO: usize = 4;
const SECTION_IMAGES: usize = 5;
const SECTION_COLORS: usize = 6;
const SECTION_ICONS: usize = 7;
pub const COLOR_COUNT: usize = 204;
pub const IMAGE_SLOTS: usize = 528;
pub const ICON_SLOTS: usize = 768;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub ignored_06: [u8; 2],
    pub ignored_08: [u8; 4],
    /// `00 00 00 00 00 00 00 00 01 01 01 01` then eight zero bytes in every
    /// file seen.
    pub ignored_74: [u8; 20],
    /// Whether an empty section carries the running offset in the table
    /// (the editor does this for 5 and 7) or `(0, 0)`.
    pub placeholders: [bool; TABLE_LEN],
    pub info: ThemeInfo,
    /// Images, in file order; `image_slots` says which UI element uses which.
    pub images: Vec<Bitmap>,
    pub image_slots: Vec<Option<usize>>,
    /// 204 colours as `0x00RRGGBB`; which slot is which is Phase 6's work.
    pub colors: Vec<u32>,
    pub icons: Vec<Bitmap>,
    pub icon_slots: Vec<Option<usize>>,
    /// Sections 0–3 and 8–12, never seen non-empty.
    pub other: [Vec<u8>; TABLE_LEN],
    pub trailing: Vec<u8>,
}

/// Section 4: the editor's Information page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeInfo {
    pub created: Time1900,
    pub modified: Time1900,
    pub ignored_10: [u8; 18],
    pub ignored_2e: [u8; 14],
    pub name: Vec<u8>,
    pub version: Vec<u8>,
    pub creator: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

/// One image or icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    /// Bits per pixel: 1, 2, 4 or 8.
    pub depth: u8,
    /// Byte 7 when byte 4 is set: the palette index drawn as transparent
    /// `[I]`.
    pub transparent: Option<u8>,
    /// Byte 7 as read when byte 4 is clear (usually 0).
    pub byte7: u8,
    /// Images only: a colour, four insets and a word; icons have none.
    pub extra: Option<ImageExtra>,
    /// `0x00RRGGBB`, 1 to 256 entries.
    pub palette: Vec<u32>,
    /// Rows of `ceil(width × depth / 8)` bytes padded to 4, MSB first.
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImageExtra {
    pub color: u32,
    /// Left, top, right, bottom `[I]`.
    pub insets: [u8; 4],
    pub ignored: u32,
}

impl Default for Theme {
    fn default() -> Self {
        let mut placeholders = [false; TABLE_LEN];
        for i in [SECTION_INFO, SECTION_IMAGES, SECTION_COLORS, SECTION_ICONS] {
            placeholders[i] = true;
        }
        Theme {
            ignored_06: [0; 2],
            ignored_08: [0; 4],
            ignored_74: [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            placeholders,
            info: ThemeInfo::default(),
            images: Vec::new(),
            image_slots: vec![None; IMAGE_SLOTS],
            colors: vec![0; COLOR_COUNT],
            icons: Vec::new(),
            icon_slots: vec![None; ICON_SLOTS],
            other: Default::default(),
            trailing: Vec::new(),
        }
    }
}

impl Bitmap {
    pub fn stride(width: u16, depth: u8) -> usize {
        (usize::from(width) * usize::from(depth)).div_ceil(32) * 4
    }

    pub fn new(
        width: u16,
        height: u16,
        depth: u8,
        palette: Vec<u32>,
        pixels: Vec<u8>,
    ) -> Result<Bitmap> {
        if !matches!(depth, 1 | 2 | 4 | 8) {
            return Err(LegacyError::Invalid("hap: depth must be 1, 2, 4 or 8"));
        }
        if palette.is_empty() || palette.len() > 256 {
            return Err(LegacyError::Invalid("hap: palette needs 1 to 256 colours"));
        }
        if pixels.len() != Self::stride(width, depth) * usize::from(height) {
            return Err(LegacyError::Invalid(
                "hap: pixel data does not match width, height and depth",
            ));
        }
        Ok(Bitmap {
            width,
            height,
            depth,
            transparent: None,
            byte7: 0,
            extra: None,
            palette,
            pixels,
        })
    }

    /// The palette index at `(x, y)`.
    pub fn pixel(&self, x: u16, y: u16) -> u8 {
        let stride = Self::stride(self.width, self.depth);
        let bit = usize::from(x) * usize::from(self.depth);
        let byte = self.pixels[usize::from(y) * stride + bit / 8];
        let shift = 8 - usize::from(self.depth) - bit % 8;
        (byte >> shift) & ((1u16 << self.depth) - 1) as u8
    }

    /// RGBA, 4 bytes per pixel, row by row; the transparent index gets
    /// alpha 0, an index past the palette draws black.
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(usize::from(self.width) * usize::from(self.height) * 4);
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.pixel(x, y);
                let c = self.palette.get(usize::from(i)).copied().unwrap_or(0);
                let a = if self.transparent == Some(i) { 0 } else { 255 };
                out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, a]);
            }
        }
        out
    }

    fn decode(r: &mut Reader, with_extra: bool) -> Result<Bitmap> {
        let width = r.u16()?;
        let height = r.u16()?;
        let flag = r.u8()?;
        let depth = r.u8()?;
        let colors = usize::from(r.u8()?) + 1;
        let byte7 = r.u8()?;
        if !matches!(depth, 1 | 2 | 4 | 8) {
            return Err(LegacyError::Invalid("hap: bitmap depth"));
        }
        let extra = if with_extra {
            Some(ImageExtra {
                color: r.u32()?,
                insets: r.array()?,
                ignored: r.u32()?,
            })
        } else {
            None
        };
        let mut palette = Vec::with_capacity(colors);
        for _ in 0..colors {
            palette.push(r.u32()?);
        }
        let pixels = r.bytes(Self::stride(width, depth) * usize::from(height))?;
        Ok(Bitmap {
            width,
            height,
            depth,
            transparent: (flag != 0).then_some(byte7),
            byte7: if flag != 0 { 0 } else { byte7 },
            extra,
            palette,
            pixels,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u16(self.width);
        w.u16(self.height);
        w.u8(u8::from(self.transparent.is_some()));
        w.u8(self.depth);
        w.u8((self.palette.len().clamp(1, 256) - 1) as u8);
        w.u8(self.transparent.unwrap_or(self.byte7));
        if let Some(e) = &self.extra {
            w.u32(e.color);
            w.bytes(&e.insets);
            w.u32(e.ignored);
        }
        for c in self.palette.iter().take(256) {
            w.u32(*c);
        }
        w.bytes(&self.pixels);
    }
}

impl ThemeInfo {
    const HEADER_LEN: usize = 0x34;

    fn decode(bytes: &[u8]) -> Result<ThemeInfo> {
        let mut r = Reader::new(bytes);
        let created = Time1900::decode(&mut r)?;
        let modified = Time1900::decode(&mut r)?;
        let ignored_10: [u8; 18] = r.array()?;
        let lens: [u8; 4] = r.array()?;
        let ignored_2e: [u8; 14] = r.array()?;
        debug_assert_eq!(r.pos(), Self::HEADER_LEN);
        let name = r.bytes(lens[0].into())?;
        let version = r.bytes(lens[1].into())?;
        let creator = r.bytes(lens[2].into())?;
        let description = r.bytes(lens[3].into())?;
        let expected = pad4_len(r.pos());
        let mut padding = r.rest();
        if padding.len() == expected && padding.iter().all(|&b| b == 0) {
            padding.clear();
        }
        Ok(ThemeInfo {
            created,
            modified,
            ignored_10,
            ignored_2e,
            name,
            version,
            creator,
            description,
            padding,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        self.created.encode(&mut w);
        self.modified.encode(&mut w);
        w.bytes(&self.ignored_10);
        let cap = |v: &Vec<u8>| v.len().min(255);
        for s in [&self.name, &self.version, &self.creator, &self.description] {
            w.u8(cap(s) as u8);
        }
        w.bytes(&self.ignored_2e);
        for s in [&self.name, &self.version, &self.creator, &self.description] {
            w.bytes(&s[..cap(s)]);
        }
        if self.padding.is_empty() {
            w.pad4(0, &[]);
        } else {
            w.bytes(&self.padding);
        }
        w.into_inner()
    }
}

fn decode_bitmaps(
    bytes: &[u8],
    slots: usize,
    with_extra: bool,
) -> Result<(Vec<Bitmap>, Vec<Option<usize>>)> {
    if bytes.is_empty() {
        return Ok((Vec::new(), vec![None; slots]));
    }
    let table_len = slots * 4;
    if bytes.len() < table_len {
        return Err(LegacyError::Invalid(
            "hap: bitmap table shorter than its slots",
        ));
    }
    let mut r = Reader::new(bytes);
    let mut offsets = Vec::with_capacity(slots);
    for _ in 0..slots {
        offsets.push(r.u32()? as usize);
    }
    let mut used: Vec<usize> = offsets.iter().copied().filter(|&o| o != 0).collect();
    used.sort_unstable();
    used.dedup();
    let mut bitmaps = Vec::with_capacity(used.len());
    for &o in &used {
        if o != r.pos() {
            return Err(LegacyError::Invalid(
                "hap: bitmap records are not contiguous",
            ));
        }
        bitmaps.push(Bitmap::decode(&mut r, with_extra)?);
    }
    if !r.is_empty() {
        return Err(LegacyError::Invalid("hap: bytes after the last bitmap"));
    }
    let slot_map = offsets
        .iter()
        .map(|&o| {
            if o == 0 {
                None
            } else {
                used.binary_search(&o).ok()
            }
        })
        .collect();
    Ok((bitmaps, slot_map))
}

fn encode_bitmaps(bitmaps: &[Bitmap], slot_map: &[Option<usize>], slots: usize) -> Vec<u8> {
    if bitmaps.is_empty() {
        return Vec::new();
    }
    let mut records = Writer::new();
    let mut starts = Vec::with_capacity(bitmaps.len());
    for b in bitmaps {
        starts.push((slots * 4 + records.pos()) as u32);
        b.encode(&mut records);
    }
    let mut w = Writer::new();
    for i in 0..slots {
        let o = slot_map
            .get(i)
            .copied()
            .flatten()
            .and_then(|k| starts.get(k))
            .copied()
            .unwrap_or(0);
        w.u32(o);
    }
    w.bytes(&records.into_inner());
    w.into_inner()
}

impl Theme {
    /// Adds an image used by the given slots; returns its index.
    pub fn add_image(&mut self, bitmap: Bitmap, slots: &[usize]) -> Result<usize> {
        let b = Bitmap {
            extra: Some(bitmap.extra.unwrap_or_default()),
            ..bitmap
        };
        Self::add(&mut self.images, &mut self.image_slots, b, slots)
    }

    pub fn add_icon(&mut self, bitmap: Bitmap, slots: &[usize]) -> Result<usize> {
        let b = Bitmap {
            extra: None,
            ..bitmap
        };
        Self::add(&mut self.icons, &mut self.icon_slots, b, slots)
    }

    fn add(
        list: &mut Vec<Bitmap>,
        map: &mut [Option<usize>],
        b: Bitmap,
        slots: &[usize],
    ) -> Result<usize> {
        if slots.iter().any(|&s| s >= map.len()) {
            return Err(LegacyError::Invalid("hap: slot out of range"));
        }
        list.push(b);
        let i = list.len() - 1;
        for &s in slots {
            map[s] = Some(i);
        }
        Ok(i)
    }

    pub fn validate(&self) -> Result<()> {
        let long = |v: &Vec<u8>| v.len() > 0xff;
        if long(&self.info.name)
            || long(&self.info.version)
            || long(&self.info.creator)
            || long(&self.info.description)
        {
            return Err(LegacyError::Invalid(
                "hap: information string longer than 255 bytes",
            ));
        }
        if self.colors.len() != COLOR_COUNT {
            return Err(LegacyError::Invalid("hap: 204 colours expected"));
        }
        for b in self.images.iter().chain(&self.icons) {
            Bitmap::new(
                b.width,
                b.height,
                b.depth,
                b.palette.clone(),
                b.pixels.clone(),
            )?;
        }
        if self.images.iter().any(|b| b.extra.is_none())
            || self.icons.iter().any(|b| b.extra.is_some())
        {
            return Err(LegacyError::Invalid(
                "hap: images carry the extra header, icons do not",
            ));
        }
        Ok(())
    }

    fn section_bytes(&self, i: usize) -> Vec<u8> {
        match i {
            SECTION_INFO => self.info.encode(),
            SECTION_IMAGES => encode_bitmaps(&self.images, &self.image_slots, IMAGE_SLOTS),
            SECTION_COLORS => {
                let mut w = Writer::new();
                for c in &self.colors {
                    w.u32(*c);
                }
                w.into_inner()
            }
            SECTION_ICONS => encode_bitmaps(&self.icons, &self.icon_slots, ICON_SLOTS),
            _ => self.other.get(i).cloned().unwrap_or_default(),
        }
    }

    /// The table as the encoder writes it.
    pub fn table(&self) -> [(u32, u32); TABLE_LEN] {
        let mut table = [(0u32, 0u32); TABLE_LEN];
        let mut offset = HEADER_LEN;
        for (i, entry) in table.iter_mut().enumerate() {
            let len = self.section_bytes(i).len();
            if len > 0 {
                *entry = (offset as u32, len as u32);
                offset += len + pad4_len(len);
            } else if self.placeholders[i] {
                *entry = (offset as u32, 0);
            }
        }
        table
    }
}

pub fn decode_hap(bytes: &[u8]) -> Result<Theme> {
    if bytes.len() >= 4
        && bytes[..4] == MAGIC[..4]
        && bytes.len() >= 6
        && bytes[4..6] != MAGIC[4..6]
    {
        return Err(LegacyError::UnsupportedVersion {
            file: "hap",
            version: u16::from_be_bytes([bytes[4], bytes[5]]),
        });
    }
    if bytes.len() < 4 || bytes[..4] != MAGIC[..4] {
        let mut found = [0u8; 8];
        for (i, b) in bytes.iter().take(8).enumerate() {
            found[i] = *b;
        }
        return Err(LegacyError::BadMagic {
            expected: MAGIC,
            found,
        });
    }
    let mut r = Reader::new(bytes);
    r.bytes(6)?;
    let ignored_06: [u8; 2] = r.array()?;
    let ignored_08: [u8; 4] = r.array()?;
    let mut table = [(0usize, 0usize); TABLE_LEN];
    for entry in &mut table {
        *entry = (r.u32()? as usize, r.u32()? as usize);
    }
    let ignored_74: [u8; 20] = r.array()?;
    debug_assert_eq!(r.pos(), HEADER_LEN);
    let mut placeholders = [false; TABLE_LEN];
    let mut sections: [Vec<u8>; TABLE_LEN] = Default::default();
    for (i, &(offset, len)) in table.iter().enumerate() {
        placeholders[i] = offset != 0;
        if len == 0 {
            continue;
        }
        let here = r.pos();
        if offset < here || offset - here > 3 {
            return Err(LegacyError::Invalid("hap: section offset"));
        }
        let pad = r.bytes(offset - here)?;
        if pad.iter().any(|&b| b != 0) {
            return Err(LegacyError::Invalid("hap: section padding"));
        }
        sections[i] = r.bytes(len)?;
    }
    let pad = pad4_len(r.pos());
    if r.remaining() >= pad {
        let p = r.bytes(pad)?;
        if p.iter().any(|&b| b != 0) {
            return Err(LegacyError::Invalid("hap: tail padding"));
        }
    }
    let trailing = r.rest();
    let info = ThemeInfo::decode(&std::mem::take(&mut sections[SECTION_INFO]))?;
    let (images, image_slots) = decode_bitmaps(
        &std::mem::take(&mut sections[SECTION_IMAGES]),
        IMAGE_SLOTS,
        true,
    )?;
    let (icons, icon_slots) = decode_bitmaps(
        &std::mem::take(&mut sections[SECTION_ICONS]),
        ICON_SLOTS,
        false,
    )?;
    let colors_bytes = std::mem::take(&mut sections[SECTION_COLORS]);
    if colors_bytes.len() != COLOR_COUNT * 4 {
        return Err(LegacyError::Decode(DecodeError::Invalid(
            "hap: colour section is not 204 entries",
        )));
    }
    let colors = colors_bytes
        .chunks_exact(4)
        .map(|c| u32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Ok(Theme {
        ignored_06,
        ignored_08,
        ignored_74,
        placeholders,
        info,
        images,
        image_slots,
        colors,
        icons,
        icon_slots,
        other: sections,
        trailing,
    })
}

pub fn encode_hap(t: &Theme) -> Vec<u8> {
    let mut w = Writer::new();
    w.bytes(&MAGIC[..6]);
    w.bytes(&t.ignored_06);
    w.bytes(&t.ignored_08);
    for (o, l) in t.table() {
        w.u32(o);
        w.u32(l);
    }
    w.bytes(&t.ignored_74);
    for i in 0..TABLE_LEN {
        let bytes = t.section_bytes(i);
        if bytes.is_empty() {
            continue;
        }
        w.pad4(0, &[]);
        w.bytes(&bytes);
    }
    w.pad4(0, &[]);
    w.bytes(&t.trailing);
    w.into_inner()
}
