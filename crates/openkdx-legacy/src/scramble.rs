//! The obfuscation of `Accounts.dat`, `KDXTrackerServ.stg` and `KDX.stg`:
//! the file is XORed, big-endian word by word, with the successive states of
//! a 32-bit linear congruential generator. Each program has its own
//! multiplier, increment and starting state, all constants, so a file's
//! scrambled form depends on nothing but its contents. A trailing partial
//! word is XORed with the leading bytes of the next state.
//!
//! Worked out from lab diffs (`docs/formats/accounts-dat.md`): a one-byte
//! change in a name flipped one byte of the file by the same XOR, two files
//! with known 32-byte account keys gave 32 keystream bytes, and those fitted
//! the ANSI C `rand()` recurrence.

/// The generator of one program's file stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stream {
    pub multiplier: u32,
    pub increment: u32,
    /// The state XORed over bytes 0..4.
    pub first: u32,
}

impl Stream {
    /// KDX Server: `Accounts.dat`.
    pub const SERVER_ACCOUNTS: Stream = Stream {
        multiplier: 0x41C6_4E6D,
        increment: 12345,
        first: 0x9ad2_2861,
    };
    /// KDX Tracker: `KDXTrackerServ.stg`.
    pub const TRACKER_SETTINGS: Stream = Stream {
        multiplier: 0x41a6_09fd,
        increment: 0x1379,
        first: 0xacb8_fa74,
    };
    /// KDX Client: `KDX.stg`.
    pub const CLIENT_SETTINGS: Stream = Stream {
        multiplier: 0x41a6_09fb,
        increment: 0x2319,
        first: 0x3566_2cb6,
    };

    fn apply(self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut state = self.first;
        for chunk in bytes.chunks(4) {
            let ks = state.to_be_bytes();
            out.extend(chunk.iter().zip(ks).map(|(b, k)| b ^ k));
            state = state
                .wrapping_mul(self.multiplier)
                .wrapping_add(self.increment);
        }
        out
    }
}

pub fn unscramble(bytes: &[u8], stream: Stream) -> Vec<u8> {
    stream.apply(bytes)
}

pub fn scramble(plain: &[u8], stream: Stream) -> Vec<u8> {
    stream.apply(plain)
}
