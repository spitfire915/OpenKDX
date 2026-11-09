//! The two challenge-table stories and how to find them in a blob.

use sha2::{Digest, Sha256};

use crate::TableError;

/// A challenge table built into the Haxial programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Story {
    /// The 1739-byte "KDX story", present in every client, server, and tracker.
    Kdx,
    /// The 3193-byte "GConsole story", present only in clients.
    Gconsole,
}

impl Story {
    pub const ALL: [Story; 2] = [Story::Kdx, Story::Gconsole];

    /// The bytes the story begins with; used to locate it inside a program.
    pub fn anchor(&self) -> &'static [u8] {
        match self {
            Story::Kdx => b"This program including the following story",
            Story::Gconsole => b"The following story is Copyright 2004",
        }
    }

    /// Length of the story in bytes.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Story::Kdx => 1739,
            Story::Gconsole => 3193,
        }
    }

    /// SHA-256 of the exact story bytes.
    pub fn sha256(&self) -> [u8; 32] {
        let hex = match self {
            Story::Kdx => "4fca7636e77e85fdde95862a6be66e12e5e031ad61add65c5e975d3daff4b9e7",
            Story::Gconsole => "b43f22222971a57a0d8bdb3a5af2747c7ff511ea88f0ba8c24e38e9edc029c36",
        };
        let mut out = [0u8; 32];
        hex::decode_to_slice(hex, &mut out).expect("constant hex");
        out
    }

    /// File name in the cache directory.
    pub fn file_name(&self) -> &'static str {
        match self {
            Story::Kdx => "kdx.bin",
            Story::Gconsole => "hgc.bin",
        }
    }

    /// Environment variable that may hold the table, base64 encoded.
    pub fn env_var(&self) -> &'static str {
        match self {
            Story::Kdx => "OPENKDX_KDX_TABLE",
            Story::Gconsole => "OPENKDX_HGC_TABLE",
        }
    }

    /// Checks that `bytes` is exactly this story.
    pub fn verify(&self, bytes: &[u8]) -> Result<(), TableError> {
        let actual: [u8; 32] = Sha256::digest(bytes).into();
        let expected = self.sha256();
        if bytes.len() == self.len() && actual == expected {
            Ok(())
        } else {
            Err(TableError::HashMismatch { expected, actual })
        }
    }
}

/// Finds `story` inside `haystack`, returning its exact bytes once verified.
pub fn extract(story: Story, haystack: &[u8]) -> Result<Vec<u8>, TableError> {
    let anchor = story.anchor();
    let mut start = 0;
    let mut found_anchor = false;
    while let Some(off) = find(&haystack[start..], anchor) {
        found_anchor = true;
        let begin = start + off;
        if let Some(candidate) = haystack.get(begin..begin + story.len())
            && story.verify(candidate).is_ok()
        {
            return Ok(candidate.to_vec());
        }
        start = begin + 1;
    }
    if found_anchor {
        // Anchor present but bytes do not hash to the story.
        let begin = find(haystack, anchor).expect("anchor was found");
        let end = (begin + story.len()).min(haystack.len());
        let actual: [u8; 32] = Sha256::digest(&haystack[begin..end]).into();
        Err(TableError::HashMismatch {
            expected: story.sha256(),
            actual,
        })
    } else {
        Err(TableError::AnchorNotFound)
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
