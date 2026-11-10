#![cfg(feature = "fetch")]
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use openkdx_table::{Cache, Source, Story, TableError, fetch_from};
use sha2::Digest;

/// Serves one HTTP response then exits. `body = None` closes right after the headers.
fn serve_once(status: &'static str, body: Option<Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let _ = s.read(&mut buf);
        match body {
            Some(b) => {
                let _ = write!(
                    s,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    b.len()
                );
                let _ = s.write_all(&b);
            }
            None => {
                let _ = write!(
                    s,
                    "HTTP/1.1 {status}\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n"
                );
                // drop: connection closes before any body arrives
            }
        }
    });
    format!("http://{addr}/archive.zip")
}

fn source(url: String, sha256: [u8; 32]) -> Source {
    Source {
        file_name: "archive.zip".into(),
        urls: vec![url],
        sha256,
        story: Story::Kdx,
    }
}

fn cache_files(dir: &std::path::Path) -> Vec<String> {
    match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => vec![],
    }
}

#[test]
fn fetch_rejects_tampered_archive() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let url = serve_once("200 OK", Some(b"definitely not the archive".to_vec()));
    let src = source(url, [0x11u8; 32]);
    let client = reqwest::blocking::Client::new();
    assert!(matches!(
        fetch_from(&[src], Story::Kdx, &cache, &client),
        Err(TableError::HashMismatch { .. })
    ));
    assert!(cache_files(cache.dir()).is_empty());
}

#[test]
fn fetch_leaves_no_partial_file_on_disconnect() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let url = serve_once("200 OK", None);
    let src = source(url, [0x22u8; 32]);
    let client = reqwest::blocking::Client::new();
    assert!(fetch_from(&[src], Story::Kdx, &cache, &client).is_err());
    assert!(cache_files(cache.dir()).is_empty());
}

#[test]
fn fetch_falls_through_to_next_url_after_404() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let bad = serve_once("404 Not Found", Some(b"nope".to_vec()));
    let also_bad = serve_once("200 OK", Some(b"wrong bytes".to_vec()));
    let src = Source {
        file_name: "archive.zip".into(),
        urls: vec![bad, also_bad],
        sha256: [0x33u8; 32],
        story: Story::Kdx,
    };
    let client = reqwest::blocking::Client::new();
    // Second URL is reached (hash mismatch), proving the 404 was skipped.
    assert!(matches!(
        fetch_from(&[src], Story::Kdx, &cache, &client),
        Err(TableError::HashMismatch { .. })
    ));
}

#[test]
fn fetch_tries_next_mirror_after_a_hash_mismatch() {
    // A mirror that serves an HTML error page with status 200 must not block
    // a later mirror that has the right bytes.
    let Some(table) = Cache::new(Cache::default_dir())
        .get(Story::Kdx)
        .ok()
        .flatten()
    else {
        eprintln!("skipped: no kdx table available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let good_archive = [&[0u8; 50][..], &table].concat();
    let sha: [u8; 32] = sha2::Sha256::digest(&good_archive).into();
    let bad = serve_once("200 OK", Some(b"<html>not found</html>".to_vec()));
    let good = serve_once("200 OK", Some(good_archive));
    let src = Source {
        file_name: "archive.zip".into(),
        urls: vec![bad, good],
        sha256: sha,
        story: Story::Kdx,
    };
    let client = reqwest::blocking::Client::new();
    assert_eq!(
        fetch_from(&[src], Story::Kdx, &cache, &client).unwrap(),
        table
    );
}
