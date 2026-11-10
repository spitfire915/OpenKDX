//! Checks that a decoded payload re-encodes to the captured bytes.

use openkdx_proto::msg::Message;
use openkdx_proto::wire::DecodeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audit {
    /// The payload re-encodes byte for byte.
    Exact,
    /// It re-encodes differently, first at `offset`.
    Mismatch {
        offset: usize,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    /// The payload did not decode at all.
    Undecoded(DecodeError),
    /// This crate has no type for the message.
    Unknown,
}

pub fn audit(message: &Result<Box<Message>, DecodeError>, payload: &[u8]) -> Audit {
    match message.as_deref() {
        Err(e) => Audit::Undecoded(e.clone()),
        Ok(Message::Unknown { .. }) => Audit::Unknown,
        Ok(m) => {
            let actual = m.encode();
            if actual == payload {
                Audit::Exact
            } else {
                let offset = actual
                    .iter()
                    .zip(payload)
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(payload.len()));
                Audit::Mismatch {
                    offset,
                    expected: payload.to_vec(),
                    actual,
                }
            }
        }
    }
}

impl Audit {
    pub fn is_exact(&self) -> bool {
        matches!(self, Audit::Exact)
    }
}

impl std::fmt::Display for Audit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Audit::Exact => f.write_str("exact"),
            Audit::Mismatch {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "MISMATCH at {offset:#x} (captured {} bytes, re-encoded {})",
                expected.len(),
                actual.len()
            ),
            Audit::Undecoded(e) => write!(f, "undecoded: {e}"),
            Audit::Unknown => f.write_str("unknown type"),
        }
    }
}
