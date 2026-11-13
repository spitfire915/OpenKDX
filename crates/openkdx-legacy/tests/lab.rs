//! Checks against files the original programs wrote in the lab, run by
//! `lab/legacy.sh`. Each test skips unless `OPENKDX_LAB_LEGACY_DIR` names
//! the snapshot directory (`legacy-samples.sh` output: one subdirectory per
//! step, each holding the server files as they were after that step).

use std::fs;
use std::path::PathBuf;

use openkdx_legacy::accounts::decode_accounts;
use openkdx_legacy::server_stg::decode_server_stg;
use openkdx_legacy::{AccountsFile, ServerSettings};

fn lab_dir() -> Option<PathBuf> {
    match std::env::var_os("OPENKDX_LAB_LEGACY_DIR") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("skipped: OPENKDX_LAB_LEGACY_DIR not set");
            None
        }
    }
}

/// Every file named `name` under the snapshot directory, in step order.
fn files(name: &str) -> Vec<(PathBuf, Vec<u8>)> {
    let Some(root) = lab_dir() else {
        return Vec::new();
    };
    let mut steps: Vec<PathBuf> = fs::read_dir(&root)
        .expect("snapshot dir")
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    steps.sort();
    let out: Vec<_> = steps
        .into_iter()
        .filter_map(|d| {
            let p = d.join(name);
            fs::read(&p).ok().map(|b| (p, b))
        })
        .collect();
    assert!(!out.is_empty(), "no {name} under {}", root.display());
    out
}

#[test]
fn lab_every_settings_file_round_trips() {
    for (path, bytes) in files("KDXServer.stg") {
        let s: ServerSettings =
            decode_server_stg(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(s.to_bytes(), bytes, "{path:?}");
    }
}

#[test]
fn lab_every_accounts_file_round_trips() {
    for (path, bytes) in files("Accounts.dat") {
        let f: AccountsFile = decode_accounts(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(
            openkdx_legacy::accounts::encode_accounts(&f),
            bytes,
            "{path:?}"
        );
        assert!(f.find(b"admin").is_some(), "{path:?}");
    }
}

#[test]
fn lab_accounts_match_cli_changes() {
    let all = files("Accounts.dat");
    if all.is_empty() {
        return;
    }
    let last = decode_accounts(&all.last().unwrap().1).unwrap();
    // legacy-samples.sh creates `legacy1` with class 1 and the folder
    // access item `Pics` before the last snapshot.
    assert_eq!(last.find(b"legacy1").expect("legacy1").class_index, 1);
    assert!(last.folder_access.iter().any(|f| f.name == b"Pics"));
}
