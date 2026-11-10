//! Challenge tables: the bytes a peer must echo to prove it is a KDX program.
//!
//! The two real tables (the KDX and GConsole stories) are Haxial's copyrighted
//! text and never ship with OpenKDX; `openkdx-table` obtains them at runtime
//! and this crate takes them as bytes. The 31-byte default is what a
//! connection made with no table uses.

/// The number of challenge bytes a proof carries.
pub const RESPONSE_LEN: usize = 20;

/// A table of bytes that a hello's `challenge` indexes into.
#[derive(Clone, PartialEq, Eq)]
pub struct ChallengeTable {
    bytes: Vec<u8>,
}

impl ChallengeTable {
    /// The table every 1.x build holds for a connection made with no story.
    pub const DEFAULT: &'static [u8] = b"All your base are belong to us!";

    pub fn new(bytes: Vec<u8>) -> Self {
        ChallengeTable { bytes }
    }

    pub fn default_table() -> Self {
        Self::new(Self::DEFAULT.to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The table's length as the programs keep it, a u16.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u16 {
        u16::try_from(self.bytes.len()).unwrap_or(u16::MAX)
    }

    /// The largest offset a response can start at: `len - 0x14`.
    pub fn max_offset(&self) -> u16 {
        self.len().saturating_sub(RESPONSE_LEN as u16)
    }

    /// The 20 bytes at `offset`, or `None` when `offset + 0x14 > len`.
    pub fn response(&self, offset: u16) -> Option<[u8; RESPONSE_LEN]> {
        if offset > self.max_offset() {
            return None;
        }
        let start = usize::from(offset);
        self.bytes.get(start..start + RESPONSE_LEN)?.try_into().ok()
    }

    pub fn verify(&self, offset: u16, response: &[u8; RESPONSE_LEN]) -> bool {
        self.response(offset).is_some_and(|r| &r == response)
    }
}

impl std::fmt::Debug for ChallengeTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChallengeTable({} bytes)", self.bytes.len())
    }
}
