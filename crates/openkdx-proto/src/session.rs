//! Session framing: the 16-byte header, its scramble, and Twofish-256 ECB
//! encryption with the account key. See the reference spec's `framing.md`.

use twofish::Twofish;
use twofish::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};

use crate::account::AccountKey;
use crate::wire::{Reader, Writer};

/// Receive caps: the largest payload each program accepts on a connection.
pub const SERVER_RECV_CAP: u64 = 0x20000;
pub const CLIENT_RECV_CAP: u64 = 0x100000;
pub const TRACKER_RECV_CAP: u64 = 0x10000;
pub const TRANSFER_RECV_CAP: u64 = 0x8000;
pub const DCC_RECV_CAP: u64 = 0x80000;
pub const VIEW_DISPLAY_RECV_CAP: u64 = 0x1000000;

const BLOCK: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error("txid or message type is zero (0x40074)")]
    ZeroTxidOrKind,
    #[error("payload length {len:#x} exceeds the cap {cap:#x} (0x40075)")]
    LengthTooLong { len: u64, cap: u64 },
    #[error("ciphertext is not the expected number of blocks")]
    BadBlockLength,
    #[error("reserved header byte is not zero")]
    ReservedNotZero,
}

/// The header that starts every session message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub txid: u32,
    pub kind: u32,
    /// Bit 0 of the flags byte.
    pub reply: bool,
    /// Payload length in bytes, before padding; 48 bits on the wire.
    pub len: u64,
    pub reserved: u8,
    /// The flags byte with bit 0 cleared.
    pub flags_rest: u8,
}

impl Header {
    pub const LEN: usize = 16;

    pub fn request(txid: u32, kind: u32, len: u64) -> Self {
        Header {
            txid,
            kind,
            reply: false,
            len,
            reserved: 0,
            flags_rest: 0,
        }
    }

    pub fn reply(txid: u32, kind: u32, len: u64) -> Self {
        Header {
            reply: true,
            ..Self::request(txid, kind, len)
        }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut w = Writer::new();
        w.u32(self.txid);
        w.u32(self.kind);
        w.u8(self.reserved);
        w.u8(self.flags_rest | u8::from(self.reply));
        w.u48(self.len & 0xffff_ffff_ffff);
        w.into_inner().try_into().expect("16 bytes")
    }

    /// Parses a header and applies the receive checks: non-zero txid and
    /// type, reserved byte zero, length within `max_len` and under 2^32.
    pub fn from_bytes(bytes: &[u8; 16], max_len: u64) -> Result<Header, SessionError> {
        let h = Self::from_bytes_unbounded(bytes)?;
        if h.len > max_len || h.len >> 32 != 0 {
            return Err(SessionError::LengthTooLong {
                len: h.len,
                cap: max_len,
            });
        }
        Ok(h)
    }

    /// Parses a transfer data header, whose length may exceed every cap.
    pub fn from_bytes_unbounded(bytes: &[u8; 16]) -> Result<Header, SessionError> {
        let mut r = Reader::new(bytes);
        let txid = r.u32().expect("16 bytes");
        let kind = r.u32().expect("16 bytes");
        let reserved = r.u8().expect("16 bytes");
        let flags = r.u8().expect("16 bytes");
        let len = r.u48().expect("16 bytes");
        if txid == 0 || kind == 0 {
            return Err(SessionError::ZeroTxidOrKind);
        }
        if reserved != 0 {
            return Err(SessionError::ReservedNotZero);
        }
        Ok(Header {
            txid,
            kind,
            reply: flags & 1 != 0,
            len,
            reserved,
            flags_rest: flags & !1,
        })
    }

    /// The payload's size on the wire: `ceil(len / 16) * 16`.
    pub fn padded_len(&self) -> usize {
        (self.len as usize).div_ceil(BLOCK) * BLOCK
    }
}

fn split_words(h: &[u8; 16]) -> [u32; 4] {
    let (w, _) = h.as_chunks::<4>();
    [
        u32::from_be_bytes(w[0]),
        u32::from_be_bytes(w[1]),
        u32::from_be_bytes(w[2]),
        u32::from_be_bytes(w[3]),
    ]
}

fn join_words(w: [u32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, word) in w.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The keyless scramble applied to a header block before encryption.
pub fn scramble(h: &mut [u8; 16]) {
    let [h0, h1, h2, h3] = split_words(h);
    *h = join_words([
        h1.rotate_left(13) ^ 0x6e7d_fd34,
        h2.rotate_right(6) ^ 0xe152_da04,
        h0.rotate_left(7) ^ 0xa95a_759b,
        h3.rotate_right(3) ^ 0x0699_2e25,
    ]);
}

/// Undoes [`scramble`].
pub fn unscramble(h: &mut [u8; 16]) {
    let [w0, w1, w2, w3] = split_words(h);
    *h = join_words([
        (w2 ^ 0xa95a_759b).rotate_right(7),
        (w0 ^ 0x6e7d_fd34).rotate_right(13),
        (w1 ^ 0xe152_da04).rotate_left(6),
        (w3 ^ 0x0699_2e25).rotate_left(3),
    ]);
}

/// The word that fills the unused end of a payload's last block.
pub fn padding_word(txid: u32) -> u32 {
    txid.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039)
}

/// Twofish-256 ECB with the account key, plus the header scramble.
#[derive(Clone)]
pub struct SessionCipher {
    cipher: Twofish,
}

impl SessionCipher {
    pub fn new(key: &AccountKey) -> Self {
        SessionCipher {
            cipher: Twofish::new(key.as_bytes().into()),
        }
    }

    /// Encrypts every 16-byte block in place. `data.len() % 16 == 0`.
    pub fn encrypt_blocks(&self, data: &mut [u8]) {
        let (blocks, _) = data.as_chunks_mut::<BLOCK>();
        for block in blocks {
            self.cipher.encrypt_block(block.into());
        }
    }

    /// Decrypts every 16-byte block in place. `data.len() % 16 == 0`.
    pub fn decrypt_blocks(&self, data: &mut [u8]) {
        let (blocks, _) = data.as_chunks_mut::<BLOCK>();
        for block in blocks {
            self.cipher.decrypt_block(block.into());
        }
    }

    /// The scrambled, encrypted header block.
    pub fn seal_header(&self, header: Header) -> [u8; 16] {
        let mut block = header.to_bytes();
        scramble(&mut block);
        self.encrypt_blocks(&mut block);
        block
    }

    /// The header block and the padded, encrypted payload blocks.
    pub fn seal(&self, header: Header, payload: &[u8]) -> Vec<u8> {
        let mut out = self.seal_header(header).to_vec();
        let padded = header.padded_len();
        let mut body = Vec::with_capacity(padded);
        let pad = padding_word(header.txid).to_be_bytes();
        for i in 0..padded {
            body.push(payload.get(i).copied().unwrap_or(pad[i % 4]));
        }
        self.encrypt_blocks(&mut body);
        out.extend_from_slice(&body);
        out
    }

    pub fn open_header(&self, block: &[u8; 16], max_len: u64) -> Result<Header, SessionError> {
        Header::from_bytes(&self.unseal_header(block), max_len)
    }

    pub fn open_header_unbounded(&self, block: &[u8; 16]) -> Result<Header, SessionError> {
        Header::from_bytes_unbounded(&self.unseal_header(block))
    }

    fn unseal_header(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut plain = *block;
        self.decrypt_blocks(&mut plain);
        unscramble(&mut plain);
        plain
    }

    /// Decrypts `header.padded_len()` bytes of payload blocks and returns the
    /// `header.len` real bytes.
    pub fn open_payload(&self, header: &Header, blocks: &[u8]) -> Result<Vec<u8>, SessionError> {
        if blocks.len() != header.padded_len() {
            return Err(SessionError::BadBlockLength);
        }
        let mut plain = blocks.to_vec();
        self.decrypt_blocks(&mut plain);
        plain.truncate(header.len as usize);
        Ok(plain)
    }
}
