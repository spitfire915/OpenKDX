use std::sync::atomic::{AtomicU32, Ordering};

/// Transaction IDs for our requests: a random start, then +1 per message,
/// never 0 (Client 1.600 does the same, `[O chat-basic]`).
#[derive(Debug)]
pub struct TxidSource(AtomicU32);

impl TxidSource {
    pub fn new(start: u32) -> Self {
        TxidSource(AtomicU32::new(start))
    }

    pub fn new_random() -> Self {
        Self::new(rand::random())
    }

    pub fn next(&self) -> u32 {
        loop {
            let id = self.0.fetch_add(1, Ordering::Relaxed);
            if id != 0 {
                return id;
            }
        }
    }
}
