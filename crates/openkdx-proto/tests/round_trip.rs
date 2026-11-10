//! The byte-exact rule for every modeled message: whatever decodes must
//! re-encode to the same bytes. Random inputs from a fixed-seed generator,
//! so the run is reproducible.

use openkdx_proto::MessageType;
use openkdx_proto::msg::Message;

/// xorshift64*: enough for test data, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }

    /// Mostly small values, so length fields often fit the input.
    fn small_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| match self.next() % 4 {
                0 => 0,
                1 => (self.next() % 8) as u8,
                _ => self.next() as u8,
            })
            .collect()
    }
}

#[test]
fn whatever_decodes_re_encodes_byte_exact() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut decoded = 0usize;
    let mut failures = Vec::new();
    for &kind in MessageType::ALL {
        if !Message::is_modeled(kind) {
            continue;
        }
        for _ in 0..400 {
            let len = (rng.next() % 300) as usize;
            let bytes = if rng.next().is_multiple_of(2) {
                rng.bytes(len)
            } else {
                rng.small_bytes(len)
            };
            if let Ok(m) = Message::decode(kind, &bytes) {
                decoded += 1;
                let again = m.encode();
                if again != bytes {
                    failures.push(format!(
                        "{kind}: {} bytes in, {} out: {}",
                        bytes.len(),
                        again.len(),
                        hex::encode(&bytes)
                    ));
                }
            }
        }
    }
    assert!(decoded > 1000, "only {decoded} random payloads decoded");
    assert!(
        failures.is_empty(),
        "{} re-encode mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_modeled_kind_is_listed() {
    // Every type in the spec's index has a struct behind it, except 184–191,
    // which are history record kinds inside 181/183, never messages.
    let unmodeled: Vec<_> = MessageType::ALL
        .iter()
        .filter(|k| !(184..=191).contains(&k.0) && !Message::is_modeled(**k))
        .collect();
    assert!(unmodeled.is_empty(), "not modeled: {unmodeled:?}");
}
