//! Import stories from a user-supplied file: a Haxial program, a zip of one,
//! or a saved table.

use std::io::Read;
use std::path::Path;

use crate::{Cache, Story, TableError, extract};

/// Scans `path` (a zip archive or any file) for every story, caches each one
/// found, and returns which. `AnchorNotFound` if the file holds none.
pub fn import(path: &Path, cache: &Cache) -> Result<Vec<Story>, TableError> {
    let data = std::fs::read(path)?;
    let found = scan_blob(&data);
    for (story, bytes) in &found {
        cache.put(*story, bytes)?;
    }
    if found.is_empty() {
        Err(TableError::AnchorNotFound)
    } else {
        Ok(found.into_iter().map(|(s, _)| s).collect())
    }
}

/// Finds every story in `data`, looking inside zip entries when `data` is a zip.
pub fn scan_blob(data: &[u8]) -> Vec<(Story, Vec<u8>)> {
    let mut found = Vec::new();
    let seen = |story: Story, bytes: Vec<u8>, found: &mut Vec<(Story, Vec<u8>)>| {
        if !found.iter().any(|(s, _)| *s == story) {
            found.push((story, bytes));
        }
    };
    if let Ok(mut zip) = zip::ZipArchive::new(std::io::Cursor::new(data)) {
        for i in 0..zip.len() {
            let Ok(mut entry) = zip.by_index(i) else {
                continue;
            };
            if entry.is_dir() {
                continue;
            }
            let mut buf = Vec::with_capacity(entry.size() as usize);
            if entry.read_to_end(&mut buf).is_err() {
                continue;
            }
            for story in Story::ALL {
                if let Ok(bytes) = extract(story, &buf) {
                    seen(story, bytes, &mut found);
                }
            }
        }
        return found;
    }
    for story in Story::ALL {
        if let Ok(bytes) = extract(story, data) {
            seen(story, bytes, &mut found);
        }
    }
    found
}
