//! The UDP datagram a server sends to register with a tracker. A separate
//! protocol from the TCP connections: its own cipher and checksum, no
//! handshake, no header, no reply. See the reference spec's `udp.md`.

use crate::wire::{DecodeError, Reader, Writer};

const HEADER_LEN: usize = 0x40;
const PROTOCOL: [u8; 4] = *b"%KDX";
const VERSION: u16 = 0x0010;
/// What the sender stores in the checksum field while computing the CRC:
/// 0xcc1ed3ca little-endian.
const CHECKSUM_PLACEHOLDER: [u8; 4] = [0xca, 0xd3, 0x1e, 0xcc];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistrationError {
    #[error("datagram is not longer than 0x40 bytes")]
    TooShort,
    #[error("protocol is not %KDX")]
    BadProtocol,
    #[error("version is not 0x0010")]
    BadVersion,
    #[error("the string lengths do not fit the datagram")]
    BadLengths,
    #[error("checksum mismatch: expected {expected:#010x}, got {actual:#010x}")]
    BadChecksum { expected: u32, actual: u32 },
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

/// One registration, as the tracker reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// Bit 0: the sender is a tracker.
    pub flags: u16,
    pub server_id: [u8; 12],
    pub server_port: u32,
    pub users: u16,
    pub string0: Vec<u8>,
    pub login: Vec<u8>,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub string4: Vec<u8>,
    pub groups: [Vec<u8>; 6],
    pub string11: Vec<u8>,
    pub string12: Vec<u8>,
    pub ignored_1c: [u8; 8],
    pub ignored_26: [u8; 13],
    /// Up to 3 bytes to a multiple of 4; Server 1.620 does not clear them.
    pub padding: Vec<u8>,
}

impl Registration {
    pub fn decode(datagram: &[u8]) -> Result<Registration, RegistrationError> {
        if datagram.len() <= HEADER_LEN {
            return Err(RegistrationError::TooShort);
        }
        let mut plain = datagram.to_vec();
        udp_xor(&mut plain);
        let mut r = Reader::new(&plain);
        let protocol: [u8; 4] = r.array()?;
        if protocol != PROTOCOL {
            return Err(RegistrationError::BadProtocol);
        }
        if r.u16()? != VERSION {
            return Err(RegistrationError::BadVersion);
        }
        let flags = r.u16()?;
        let checksum = r.u32()?;
        let server_id: [u8; 12] = r.array()?;
        let server_port = r.u32()?;
        let ignored_1c: [u8; 8] = r.array()?;
        let users = r.u16()?;
        let ignored_26: [u8; 13] = r.array()?;
        let lens: [u8; 13] = r.array()?;
        debug_assert_eq!(r.pos(), HEADER_LEN);
        let total: usize = lens.iter().map(|&l| usize::from(l)).sum();
        if lens.iter().all(|&l| l == 0) || HEADER_LEN + total > plain.len() {
            return Err(RegistrationError::BadLengths);
        }

        let mut check = plain.clone();
        check[8..12].copy_from_slice(&CHECKSUM_PLACEHOLDER);
        let actual = udp_crc(&check);
        if actual != checksum {
            return Err(RegistrationError::BadChecksum {
                expected: checksum,
                actual,
            });
        }

        let mut s = |n: u8| r.bytes(usize::from(n));
        let string0 = s(lens[0])?;
        let login = s(lens[1])?;
        let name = s(lens[2])?;
        let description = s(lens[3])?;
        let string4 = s(lens[4])?;
        let groups = [
            s(lens[5])?,
            s(lens[6])?,
            s(lens[7])?,
            s(lens[8])?,
            s(lens[9])?,
            s(lens[10])?,
        ];
        let string11 = s(lens[11])?;
        let string12 = s(lens[12])?;
        let padding = r.rest();
        Ok(Registration {
            flags,
            server_id,
            server_port,
            users,
            string0,
            login,
            name,
            description,
            string4,
            groups,
            string11,
            string12,
            ignored_1c,
            ignored_26,
            padding,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let strings: [&[u8]; 13] = [
            &self.string0,
            &self.login,
            &self.name,
            &self.description,
            &self.string4,
            &self.groups[0],
            &self.groups[1],
            &self.groups[2],
            &self.groups[3],
            &self.groups[4],
            &self.groups[5],
            &self.string11,
            &self.string12,
        ];
        let mut w = Writer::new();
        w.bytes(&PROTOCOL);
        w.u16(VERSION);
        w.u16(self.flags);
        w.bytes(&CHECKSUM_PLACEHOLDER);
        w.bytes(&self.server_id);
        w.u32(self.server_port);
        w.bytes(&self.ignored_1c);
        w.u16(self.users);
        w.bytes(&self.ignored_26);
        for s in strings {
            w.u8(s.len().min(0xff) as u8);
        }
        for s in strings {
            w.bytes(&s[..s.len().min(0xff)]);
        }
        w.pad4(0, &self.padding);
        let mut out = w.into_inner();
        let crc = udp_crc(&out);
        out[8..12].copy_from_slice(&crc.to_be_bytes());
        udp_xor(&mut out);
        out
    }
}

/// XORs each big-endian word with the LCG stream `s = s * 0x41d28485 + 0x322b`
/// from 0xa5a16c4a. Whole words only; it is its own inverse.
pub fn udp_xor(data: &mut [u8]) {
    let mut s: u32 = 0xa5a1_6c4a;
    let (words, _) = data.as_chunks_mut::<4>();
    for word in words {
        *word = (u32::from_be_bytes(*word) ^ s).to_be_bytes();
        s = s.wrapping_mul(0x41d2_8485).wrapping_add(0x322b);
    }
}

/// CRC-32, polynomial 0x04c11db7 MSB first, initial value 0xdeadbeef, no final XOR.
pub fn udp_crc(data: &[u8]) -> u32 {
    let mut reg: u32 = 0xdead_beef;
    for &b in data {
        reg ^= u32::from(b) << 24;
        for _ in 0..8 {
            reg = if reg & 0x8000_0000 != 0 {
                (reg << 1) ^ 0x04c1_1db7
            } else {
                reg << 1
            };
        }
    }
    reg
}
