//! Fetch the smallest Haxial archive that holds each story.

use sha2::{Digest, Sha256};

use crate::import::scan_blob;
use crate::{Cache, Story, TableError};

/// An archive known to hold a story, with mirrors and its exact SHA-256.
#[derive(Debug, Clone)]
pub struct Source {
    pub file_name: String,
    pub urls: Vec<String>,
    pub sha256: [u8; 32],
    pub story: Story,
}

fn hex32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    hex::decode_to_slice(s, &mut out).expect("constant hex");
    out
}

/// The default sources: Tracker 1.110 (Win, 248 KB) for the KDX story and
/// Client 1.600 (Win, 793 KB) for the GConsole story. Hashes from
/// scottanderson/kdx `vendor/MANIFEST.md`, verified against preterhuman.net.
///
/// Note the file names differ by mirror: preterhuman.net serves the client as
/// `KDXClient-Win.zip`. kdx.technowiki.info's client is a patched "unlocked"
/// build with a different hash, so it is not a source for the stock archive.
pub fn sources() -> Vec<Source> {
    vec![
        Source {
            file_name: "KDXTrackerServ1110-Win.zip".into(),
            urls: vec![
                "https://preterhuman.net/download/KDXTrackerServ1110-Win.zip".into(),
                "https://kdx.technowiki.info/downloads/KDXTrackerServ1110-Win.zip".into(),
                "https://web.archive.org/web/20051014065628id_/http://www.haxial.com:80/download/KDXTrackerServ1110-Win.zip".into(),
            ],
            sha256: hex32("8f50ef0cd4f82fc5b071340932c498a9df3556ed6cff6f23ecd6c36c43683583"),
            story: Story::Kdx,
        },
        Source {
            file_name: "KDXClient1600-Win.zip".into(),
            urls: vec![
                "https://preterhuman.net/download/KDXClient-Win.zip".into(),
                "https://web.archive.org/web/20051014063319id_/http://www.haxial.com:80/download/KDXClient1600-Win.zip".into(),
            ],
            sha256: hex32("cc6ec09bad1969e354ed5778ad594a0f8639ae8e3aa05e169916cc4b8d4f038b"),
            story: Story::Gconsole,
        },
    ]
}

/// Fetches `story` using the default sources and caches it.
pub fn fetch(
    story: Story,
    cache: &Cache,
    client: &reqwest::blocking::Client,
) -> Result<Vec<u8>, TableError> {
    fetch_from(&sources(), story, cache, client)
}

/// Fetches `story` from the first source that serves it: tries each URL in
/// order, verifies the archive's SHA-256, scans it for the story, and caches
/// everything found. Nothing is written to the cache unless the archive hash
/// matches and the story is present.
pub fn fetch_from(
    sources: &[Source],
    story: Story,
    cache: &Cache,
    client: &reqwest::blocking::Client,
) -> Result<Vec<u8>, TableError> {
    let Some(src) = sources.iter().find(|s| s.story == story) else {
        return Err(TableError::NoSource(story));
    };
    let mut last_err: Option<TableError> = None;
    for url in &src.urls {
        let body = match download(client, url) {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };
        let actual: [u8; 32] = Sha256::digest(&body).into();
        if actual != src.sha256 {
            last_err = Some(TableError::HashMismatch {
                expected: src.sha256,
                actual,
            });
            continue;
        }
        let found = scan_blob(&body);
        let mut wanted = None;
        for (s, bytes) in found {
            cache.put(s, &bytes)?;
            if s == story {
                wanted = Some(bytes);
            }
        }
        return wanted.ok_or(TableError::AnchorNotFound);
    }
    Err(last_err.unwrap_or(TableError::NoSource(story)))
}

fn download(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, TableError> {
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(TableError::HttpStatus {
            url: url.to_string(),
            status: resp.status().as_u16(),
        });
    }
    Ok(resp.bytes()?.to_vec())
}
