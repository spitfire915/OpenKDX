//! Byte-level reading and writing of wire structs. Big-endian throughout.

/// Why a payload could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("truncated: needed {needed} more bytes at offset {offset}")]
    Truncated { offset: usize, needed: usize },
    #[error("{0}")]
    Invalid(&'static str),
    #[error("{0} trailing bytes")]
    Trailing(usize),
}

/// Bytes of padding that bring `len` up to a multiple of 4.
pub fn pad4_len(len: usize) -> usize {
    (4 - len % 4) % 4
}

/// A cursor over a byte slice.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.remaining() < n {
            return Err(DecodeError::Truncated {
                offset: self.pos,
                needed: n - self.remaining(),
            });
        }
        let out = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    /// A 48-bit big-endian number, as in the header length.
    pub fn u48(&mut self) -> Result<u64, DecodeError> {
        let b = self.take(6)?;
        let mut out = [0u8; 8];
        out[2..].copy_from_slice(b);
        Ok(u64::from_be_bytes(out))
    }

    pub fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// `n` bytes, also used for `str` fields, which are raw bytes on the wire.
    pub fn bytes(&mut self, n: usize) -> Result<Vec<u8>, DecodeError> {
        Ok(self.take(n)?.to_vec())
    }

    /// Everything left.
    pub fn rest(&mut self) -> Vec<u8> {
        let out = self.bytes[self.pos..].to_vec();
        self.pos = self.bytes.len();
        out
    }

    /// The `pad4` bytes of a struct that began at `struct_start`: zero to three
    /// bytes up to a multiple of 4. If the input ends sooner, what is there.
    pub fn pad4(&mut self, struct_start: usize) -> Result<Vec<u8>, DecodeError> {
        let want = pad4_len(self.pos - struct_start);
        let n = want.min(self.remaining());
        self.bytes(n)
    }

    /// Succeeds only if every byte was consumed.
    pub fn finish(self) -> Result<(), DecodeError> {
        match self.remaining() {
            0 => Ok(()),
            n => Err(DecodeError::Trailing(n)),
        }
    }
}

/// Appends wire fields to a byte vector.
#[derive(Debug, Default, Clone)]
pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pos(&self) -> usize {
        self.0.len()
    }

    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }

    pub fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    pub fn u48(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes()[2..]);
    }

    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }

    /// Writes the padding of a struct that began at `struct_start`: the bytes
    /// kept from decoding, or zeros, up to a multiple of 4.
    pub fn pad4(&mut self, struct_start: usize, kept: &[u8]) {
        let want = pad4_len(self.pos() - struct_start);
        for i in 0..want {
            self.0.push(kept.get(i).copied().unwrap_or(0));
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u48_reads_six_bytes_big_endian() {
        let mut r = Reader::new(&[0, 1, 0, 0, 0, 2]);
        assert_eq!(r.u48().unwrap(), 0x0001_0000_0002);
        assert!(r.is_empty());
    }

    #[test]
    fn truncated_reports_offset_and_need() {
        let mut r = Reader::new(&[1, 2]);
        assert_eq!(
            r.u32(),
            Err(DecodeError::Truncated {
                offset: 0,
                needed: 2
            })
        );
        assert_eq!(r.pos(), 0);
    }

    #[test]
    fn pad4_from_struct_start() {
        let data = [9, 9, 1, 2, 3, 4, 5, 0xaa, 0xbb, 0xcc, 7];
        let mut r = Reader::new(&data);
        r.u16().unwrap();
        let start = r.pos();
        r.bytes(5).unwrap();
        assert_eq!(r.pad4(start).unwrap(), vec![0xaa, 0xbb, 0xcc]);
        assert_eq!(r.remaining(), 1);
    }

    #[test]
    fn pad4_at_end_of_input_is_short() {
        let mut r = Reader::new(&[1, 2, 3, 4, 5, 0xaa]);
        r.bytes(5).unwrap();
        assert_eq!(r.pad4(0).unwrap(), vec![0xaa]);
        assert!(r.is_empty());
    }

    #[test]
    fn finish_reports_trailing() {
        let mut r = Reader::new(&[1, 2, 3]);
        r.u8().unwrap();
        assert_eq!(r.finish(), Err(DecodeError::Trailing(2)));
        assert_eq!(Reader::new(&[]).finish(), Ok(()));
    }

    #[test]
    fn writer_roundtrip_all_widths() {
        let mut w = Writer::new();
        w.u8(1);
        w.u16(0x0203);
        w.u32(0x0405_0607);
        w.u48(0x0809_0a0b_0c0d);
        w.u64(0x0e0f_1011_1213_1415);
        w.bytes(b"ab");
        w.pad4(0, &[0xee]);
        let bytes = w.into_inner();
        assert_eq!(bytes.len(), 24);
        let mut r = Reader::new(&bytes);
        assert_eq!(r.u8().unwrap(), 1);
        assert_eq!(r.u16().unwrap(), 0x0203);
        assert_eq!(r.u32().unwrap(), 0x0405_0607);
        assert_eq!(r.u48().unwrap(), 0x0809_0a0b_0c0d);
        assert_eq!(r.u64().unwrap(), 0x0e0f_1011_1213_1415);
        assert_eq!(r.bytes(2).unwrap(), b"ab");
        assert_eq!(r.pad4(0).unwrap(), vec![0xee]);
        r.finish().unwrap();
    }
}
