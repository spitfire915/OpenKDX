//! The handshake: two hellos and two proofs, in cleartext under an XOR stream.
//! It is also the login; see the reference spec's `handshake.md`.

use sha1::{Digest, Sha1};
use twofish::Twofish;
use twofish::cipher::{BlockCipherEncrypt, KeyInit};

use crate::account::AccountKey;
use crate::challenge::{ChallengeTable, RESPONSE_LEN};
use crate::wire::{DecodeError, Reader, Writer};
use crate::xor::xor_stream;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("incomplete: {needed} more bytes needed")]
    Incomplete { needed: usize },
    #[error("unknown protocol or sub-protocol (0x40071)")]
    UnknownProtocol,
    #[error("the peer's protocol version is too old (0x40072)")]
    TooOld,
    #[error("the peer's protocol version is too new (0x40073)")]
    TooNew,
    #[error("hello longer than 0x800 bytes (0x40075)")]
    TooLong,
    #[error("invalid proof status {0:#x}")]
    InvalidStatus(u8),
    #[error("the peer is a GConsole host and no GConsole challenge table is available")]
    MissingGConsoleTable,
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

impl HandshakeError {
    pub const CODE_AUTH_FAILED: u32 = 0x40070;
    pub const CODE_UNKNOWN_PROTOCOL: u32 = 0x40071;
    pub const CODE_TOO_OLD: u32 = 0x40072;
    pub const CODE_TOO_NEW: u32 = 0x40073;
    pub const CODE_INVALID_DATA: u32 = 0x40074;
    pub const CODE_LIMIT_EXCEEDED: u32 = 0x40075;
}

/// What the sender of a hello is, from its sub-version.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Role {
    Client = 0x10,
    Server = 0x20,
    Tracker = 0x30,
    GConsole = 0x40,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubProtocol {
    /// `%KDX`: client, server, and tracker.
    Kdx,
    /// `%HGC`: a GConsole peer.
    Hgc,
}

impl SubProtocol {
    pub fn tag(self) -> [u8; 4] {
        match self {
            SubProtocol::Kdx => *b"%KDX",
            SubProtocol::Hgc => *b"%HGC",
        }
    }

    fn from_tag(tag: [u8; 4]) -> Option<Self> {
        match &tag {
            b"%KDX" => Some(SubProtocol::Kdx),
            b"%HGC" => Some(SubProtocol::Hgc),
            _ => None,
        }
    }
}

const PROTOCOL: [u8; 4] = *b"%TXP";
const HELLO_FIXED: usize = 0x24;
const NAME_KEPT: usize = 31;

/// The first message from each side.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Hello {
    pub sub_protocol: SubProtocol,
    pub version: u8,
    pub sub_version: u8,
    pub challenge: u16,
    pub nonce: [u8; 12],
    /// The login name from a client: empty for guest, `\0` + u32 session ID to
    /// attach a transfer connection. All `name_len` bytes as sent; the server
    /// uses the first 31 ([`login`](Self::login)).
    pub name: Vec<u8>,
    pub pad_len: u8,
    pub skipped_len: u16,
    pub ignored: [u8; 4],
    /// The `pad_len` bytes before the name, kept so a hello re-encodes exactly.
    pub pad: Vec<u8>,
    /// The `skipped_len` bytes after them, likewise.
    pub skipped: Vec<u8>,
    pub padding: Vec<u8>,
}

impl Hello {
    pub const MAX_LEN: usize = 0x800;

    /// `(name_len + pad_len + skipped_len + 0x27) & !3`.
    pub fn wire_len(&self) -> usize {
        Self::wire_len_of(self.name.len().min(0xff), self.pad_len, self.skipped_len)
    }

    fn wire_len_of(name_len: usize, pad_len: u8, skipped_len: u16) -> usize {
        (name_len + usize::from(pad_len) + usize::from(skipped_len) + 0x27) & !3
    }

    /// Decodes a hello from the start of `bytes`, returning it and the number
    /// of bytes it took.
    pub fn decode(bytes: &[u8]) -> Result<(Hello, usize), HandshakeError> {
        if bytes.len() < HELLO_FIXED {
            return Err(HandshakeError::Incomplete {
                needed: HELLO_FIXED - bytes.len(),
            });
        }
        let key = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mut fixed = bytes[4..HELLO_FIXED].to_vec();
        xor_stream(key, &mut fixed);
        let mut r = Reader::new(&fixed);
        let protocol: [u8; 4] = r.array()?;
        let sub_protocol = SubProtocol::from_tag(r.array()?);
        let version = r.u8()?;
        let sub_version = r.u8()?;
        let pad_len = r.u8()?;
        let name_len = r.u8()?;
        let ignored: [u8; 4] = r.array()?;
        let skipped_len = r.u16()?;
        let challenge = r.u16()?;
        let nonce: [u8; 12] = r.array()?;
        r.finish()?;

        if protocol != PROTOCOL {
            return Err(HandshakeError::UnknownProtocol);
        }
        let Some(sub_protocol) = sub_protocol else {
            return Err(HandshakeError::UnknownProtocol);
        };
        match version {
            0 => return Err(HandshakeError::TooOld),
            1 => {}
            _ => return Err(HandshakeError::TooNew),
        }
        let total = Self::wire_len_of(usize::from(name_len), pad_len, skipped_len);
        if total > Self::MAX_LEN {
            return Err(HandshakeError::TooLong);
        }
        if bytes.len() < total {
            return Err(HandshakeError::Incomplete {
                needed: total - bytes.len(),
            });
        }
        let mut body = bytes[HELLO_FIXED..total].to_vec();
        // The stream continues from where the fixed part left it.
        let mut whole = bytes[4..total].to_vec();
        xor_stream(key, &mut whole);
        body.copy_from_slice(&whole[HELLO_FIXED - 4..]);
        let mut r = Reader::new(&body);
        let pad = r.bytes(usize::from(pad_len))?;
        let skipped = r.bytes(usize::from(skipped_len))?;
        let name = r.bytes(usize::from(name_len))?;
        let padding = r.rest();
        Ok((
            Hello {
                sub_protocol,
                version,
                sub_version,
                challenge,
                nonce,
                name,
                pad_len,
                skipped_len,
                ignored,
                pad,
                skipped,
                padding,
            },
            total,
        ))
    }

    /// Encodes the hello under the XOR stream seeded with `key`.
    pub fn encode(&self, key: u32) -> Vec<u8> {
        let name_len = self.name.len().min(0xff);
        let mut w = Writer::new();
        w.bytes(&PROTOCOL);
        w.bytes(&self.sub_protocol.tag());
        w.u8(self.version);
        w.u8(self.sub_version);
        w.u8(self.pad_len);
        w.u8(name_len as u8);
        w.bytes(&self.ignored);
        w.u16(self.skipped_len);
        w.u16(self.challenge);
        w.bytes(&self.nonce);
        let mut gap = vec![0u8; usize::from(self.pad_len) + usize::from(self.skipped_len)];
        for (dst, src) in gap
            .iter_mut()
            .zip(self.pad.iter().chain(self.skipped.iter()))
        {
            *dst = *src;
        }
        w.bytes(&gap);
        w.bytes(&self.name[..name_len]);
        w.pad4(0, &self.padding);
        let mut body = w.into_inner();
        xor_stream(key, &mut body);
        let mut out = key.to_be_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }

    /// The login the server acts on: the first 31 bytes of the name.
    pub fn login(&self) -> &[u8] {
        &self.name[..self.name.len().min(NAME_KEPT)]
    }

    pub fn role(&self) -> Option<Role> {
        match (self.sub_protocol, self.sub_version) {
            (SubProtocol::Kdx, 0x10) => Some(Role::Client),
            (SubProtocol::Kdx, 0x20) => Some(Role::Server),
            (SubProtocol::Kdx, 0x30) => Some(Role::Tracker),
            (SubProtocol::Hgc, 0x40) => Some(Role::GConsole),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ProofStatus {
    Ok = 0x10,
    /// Server 1.620 answers a bad proof with this (lab, Phase 2); the
    /// reference lists it only for the 0.9x handshake.
    Failed = 1,
    UnknownProtocol = 2,
    TooOld = 3,
    TooNew = 4,
}

impl ProofStatus {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x10 => Some(ProofStatus::Ok),
            1 => Some(ProofStatus::Failed),
            2 => Some(ProofStatus::UnknownProtocol),
            3 => Some(ProofStatus::TooOld),
            4 => Some(ProofStatus::TooNew),
            _ => None,
        }
    }
}

/// The second message from each side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Proof {
    /// Proves the account key; its first 4 bytes are also the XOR key.
    pub auth: [u8; 20],
    /// The 20 challenge-table bytes at the offset the peer's hello gave.
    pub response: [u8; RESPONSE_LEN],
    pub status: ProofStatus,
    pub ignored: [u8; 3],
}

impl Proof {
    pub const LEN: usize = 44;

    pub fn decode(bytes: &[u8; 44]) -> Result<Proof, HandshakeError> {
        let key = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mut plain = *bytes;
        xor_stream(key, &mut plain[4..]);
        let mut r = Reader::new(&plain);
        let auth: [u8; 20] = r.array()?;
        let response: [u8; 20] = r.array()?;
        let status = r.u8()?;
        let ignored: [u8; 3] = r.array()?;
        let status = ProofStatus::from_u8(status).ok_or(HandshakeError::InvalidStatus(status))?;
        Ok(Proof {
            auth,
            response,
            status,
            ignored,
        })
    }

    pub fn encode(&self) -> [u8; 44] {
        let mut out = [0u8; 44];
        out[..20].copy_from_slice(&self.auth);
        out[20..40].copy_from_slice(&self.response);
        out[40] = self.status as u8;
        out[41..].copy_from_slice(&self.ignored);
        let key = u32::from_be_bytes([out[0], out[1], out[2], out[3]]);
        xor_stream(key, &mut out[4..]);
        out
    }

    /// Our proof for a peer whose hello carried `peer_nonce` and
    /// `peer_challenge`. `None` when the challenge is past the table's end.
    pub fn compute(
        key: &AccountKey,
        peer_nonce: &[u8; 12],
        table: &ChallengeTable,
        peer_challenge: u16,
    ) -> Option<Proof> {
        Some(Proof {
            auth: proof_auth(key, peer_nonce),
            response: table.response(peer_challenge)?,
            status: ProofStatus::Ok,
            ignored: [0; 3],
        })
    }

    /// The proof a side sends when it rejects the peer's hello.
    pub fn failure(peer_nonce: &[u8; 12], status: ProofStatus) -> Proof {
        let word = u32::from_be_bytes([peer_nonce[4], peer_nonce[5], peer_nonce[6], peer_nonce[7]])
            ^ 0x201f_7790;
        let mut auth = [0u8; 20];
        auth[..4].copy_from_slice(&word.to_be_bytes());
        Proof {
            auth,
            response: [0; RESPONSE_LEN],
            status,
            ignored: [0; 3],
        }
    }
}

/// The table that answers `peer`'s challenge. A connection starts with the KDX
/// story; after an `%HGC` hello the client switches to the GConsole story, so
/// it picked its own challenge from the KDX story but answers from the other.
pub fn table_for_peer<'a>(
    peer: &Hello,
    kdx: &'a ChallengeTable,
    hgc: Option<&'a ChallengeTable>,
) -> Result<&'a ChallengeTable, HandshakeError> {
    match peer.sub_protocol {
        SubProtocol::Kdx => Ok(kdx),
        SubProtocol::Hgc => match peer.sub_version {
            0x40 => hgc.ok_or(HandshakeError::MissingGConsoleTable),
            v if v < 0x40 => Err(HandshakeError::TooOld),
            _ => Err(HandshakeError::TooNew),
        },
    }
}

/// `Twofish-256(key).encrypt(SHA-1(interleave(key, peer_nonce))[0..16]) || SHA-1(...)[16..20]`.
pub fn proof_auth(key: &AccountKey, peer_nonce: &[u8; 12]) -> [u8; 20] {
    let s = key.as_bytes();
    let n = peer_nonce;
    let mut input = Vec::with_capacity(44);
    input.extend_from_slice(&s[0..4]);
    input.extend_from_slice(&n[0..4]);
    input.extend_from_slice(&s[4..12]);
    input.extend_from_slice(&n[4..8]);
    input.extend_from_slice(&s[12..20]);
    input.extend_from_slice(&n[8..12]);
    input.extend_from_slice(&s[20..32]);
    let digest = Sha1::digest(&input);
    let cipher = Twofish::new(s.into());
    let mut block = twofish::cipher::Block::<Twofish>::default();
    block.copy_from_slice(&digest[..16]);
    cipher.encrypt_block(&mut block);
    let mut auth = [0u8; 20];
    auth[..16].copy_from_slice(&block);
    auth[16..].copy_from_slice(&digest[16..]);
    auth
}
