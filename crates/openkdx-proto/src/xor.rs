//! The XOR stream that covers handshake messages.

/// XORs each 32-bit big-endian word of `body` with the stream
/// `k = (k << 1) + 0x4878`, seeded with `key`. The stream runs on through the
/// whole body. `body.len()` must be a multiple of 4; a trailing partial word
/// is left alone.
pub fn xor_stream(key: u32, body: &mut [u8]) {
    let mut k = key;
    let (words, _) = body.as_chunks_mut::<4>();
    for word in words {
        k = (k << 1).wrapping_add(0x4878);
        *word = (u32::from_be_bytes(*word) ^ k).to_be_bytes();
    }
}
