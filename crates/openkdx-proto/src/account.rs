//! The 32-byte account key that both sides derive from the password.

use sha1::{Digest, Sha1};

/// The key shared by a client and a server for one account: the proof is
/// computed from it and the session is encrypted with it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountKey(pub [u8; 32]);

impl AccountKey {
    /// The key both sides use for an empty password (a guest login). It is a
    /// fixed constant, not what the formula gives for an empty input.
    pub const GUEST: AccountKey = AccountKey([
        0xda, 0x39, 0xa3, 0xee, 0xca, 0xae, 0xbe, 0x6e, 0x5e, 0x6b, 0x4b, 0x0d, 0xba, 0x8e, 0x29,
        0xba, 0x32, 0x55, 0xbf, 0xef, 0x18, 0xa2, 0xb5, 0xc4, 0x95, 0x60, 0x18, 0x90, 0xaf, 0xd8,
        0x07, 0x09,
    ]);

    /// `SHA-1(password) || SHA-1(password)[0..8] || BE32(FNV-1(password, seed 1))`,
    /// or [`GUEST`](Self::GUEST) for an empty password.
    pub fn from_password(password: &[u8]) -> Self {
        if password.is_empty() {
            return Self::GUEST;
        }
        let digest = Sha1::digest(password);
        let mut key = [0u8; 32];
        key[..20].copy_from_slice(&digest);
        key[20..28].copy_from_slice(&digest[..8]);
        key[28..].copy_from_slice(&fnv1(password, 1).to_be_bytes());
        AccountKey(key)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for AccountKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccountKey(")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        write!(f, ")")
    }
}

/// FNV-1 (multiply, then XOR) with the 32-bit prime, from `seed`.
pub fn fnv1(bytes: &[u8], seed: u32) -> u32 {
    bytes
        .iter()
        .fold(seed, |h, &b| h.wrapping_mul(0x0100_0193) ^ u32::from(b))
}
