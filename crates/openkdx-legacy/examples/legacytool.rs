//! A small command-line tool over `openkdx-legacy`, for the lab checks and
//! for looking at files by hand. No dependencies beyond the crate.
//!
//!   legacytool dump <file>                       decode and print a summary
//!   legacytool unscramble <file> <server|tracker|client>   plaintext to stdout
//!   legacytool add-account <dir> <login> <password> [class]
//!   legacytool set-name <dir> <name>             server name in KDXServer.stg
//!   legacytool rewrite <dir>                     load and save every known file

use std::io::Write;
use std::path::Path;

use openkdx_legacy::accounts::decode_accounts;
use openkdx_legacy::scramble::{Stream, unscramble};
use openkdx_legacy::server_stg::decode_server_stg;
use openkdx_legacy::{LegacyDir, ServerFile};
use openkdx_proto::AccountKey;
use openkdx_proto::msg::accounts::{Account, Privileges};
use openkdx_proto::msg::common::Time1900;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match strs.as_slice() {
        ["dump", file] => dump(Path::new(file)),
        ["unscramble", file, which] => {
            let stream = match *which {
                "server" => Stream::SERVER_ACCOUNTS,
                "tracker" => Stream::TRACKER_SETTINGS,
                "client" => Stream::CLIENT_SETTINGS,
                _ => return usage(),
            };
            let bytes = std::fs::read(file).expect("read");
            std::io::stdout()
                .write_all(&unscramble(&bytes, stream))
                .expect("write");
            Ok(())
        }
        ["add-account", dir, login, password, rest @ ..] => {
            let class: u8 = rest.first().map_or(1, |c| c.parse().expect("class index"));
            add_account(Path::new(dir), login.as_bytes(), password.as_bytes(), class)
        }
        ["set-name", dir, name] => set_name(Path::new(dir), name.as_bytes()),
        ["rewrite", dir] => rewrite(Path::new(dir)),
        _ => return usage(),
    };
    if let Err(e) = result {
        eprintln!("legacytool: {e}");
        std::process::exit(1);
    }
}

fn usage() {
    eprintln!(
        "usage: legacytool dump FILE | unscramble FILE server|tracker|client | add-account DIR LOGIN PASSWORD [CLASS] | set-name DIR NAME | rewrite DIR"
    );
    std::process::exit(2);
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn dump(path: &Path) -> openkdx_legacy::Result<()> {
    let bytes = std::fs::read(path).expect("read");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match name {
        _ if name == "Accounts.dat" || decode_accounts(&bytes).is_ok() => {
            let f = decode_accounts(&bytes)?;
            println!(
                "Accounts.dat v{}: {} classes, {} accounts, {} folder access items",
                f.version,
                f.classes.len(),
                f.accounts.len(),
                f.folder_access.len()
            );
            for c in &f.classes {
                println!(
                    "  class {:2} {:?} speed {} flood {:#x}",
                    c.class_index,
                    lossy(&c.name),
                    c.speed_limit,
                    c.flood_protection
                );
            }
            for a in &f.accounts {
                println!(
                    "  account {:?} name {:?} class {} editor {:?} last_login {}",
                    lossy(&a.login),
                    lossy(&a.name),
                    a.class_index,
                    lossy(&a.editor),
                    a.last_login.0
                );
            }
            for x in &f.folder_access {
                println!(
                    "  folder access {:?} icon {:#x} owner {:?} {:?}",
                    lossy(&x.name),
                    x.icon,
                    lossy(&x.owner),
                    lossy(&x.description)
                );
            }
        }
        _ if bytes.starts_with(b"%KDXSSTG") => {
            let s = decode_server_stg(&bytes)?;
            println!(
                "KDXServer.stg: name {:?} description {:?} address {:?} port {} alt {} ft {} speed {} history {:#x}/{} flags {:#x}",
                lossy(&s.main.name),
                lossy(&s.main.description),
                lossy(&s.main.address),
                s.main.port,
                s.main.alternate_port,
                s.main.file_transfer_port,
                s.main.total_outgoing_speed_limit,
                s.main.history_kinds,
                s.main.history_max_file_size,
                s.main.flags
            );
            for t in &s.trackers {
                println!(
                    "  tracker {:?} {:?} login {:?} groups {:?}",
                    lossy(&t.name),
                    lossy(&t.address),
                    lossy(&t.login),
                    lossy(&t.groups)
                );
            }
            for a in &s.allow_deny {
                println!(
                    "  {} {:?} {:?} expires {}",
                    if a.allow { "allow" } else { "deny" },
                    lossy(&a.address),
                    lossy(&a.name),
                    a.expires.0
                );
            }
            if let Some(g) = &s.greeting {
                println!("  greeting {:?}", lossy(&g.text));
            }
        }
        _ => {
            println!(
                "{}: {} bytes, not a format legacytool dumps",
                path.display(),
                bytes.len()
            );
        }
    }
    Ok(())
}

fn add_account(dir: &Path, login: &[u8], password: &[u8], class: u8) -> openkdx_legacy::Result<()> {
    let store = LegacyDir::open(dir)?;
    let mut f = store.load_accounts()?.expect("Accounts.dat");
    let now = Time1900::from_unix_ms(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64),
    );
    let template = f.accounts.iter().find(|a| a.class_index == class).cloned();
    let account = Account {
        privileges: Privileges::NONE,
        privilege_mask: Privileges::NONE,
        key: *AccountKey::from_password(password).as_bytes(),
        created: now,
        modified: now,
        last_login: Time1900(0),
        ignored_58: template.as_ref().map_or([0; 30], |t| t.ignored_58),
        class_index: class,
        color: Default::default(),
        ignored_7e: [0; 2],
        login: login.to_vec(),
        name: login.to_vec(),
        third: Vec::new(),
        editor: b"legacytool".to_vec(),
        padding: Vec::new(),
    };
    f.upsert(account);
    store.save_accounts(&f)
}

fn set_name(dir: &Path, name: &[u8]) -> openkdx_legacy::Result<()> {
    let store = LegacyDir::open(dir)?;
    let mut s = store.load_settings()?.expect("KDXServer.stg");
    s.main.name = name.to_vec();
    store.save_settings(&s)
}

fn rewrite(dir: &Path) -> openkdx_legacy::Result<()> {
    let store = LegacyDir::open(dir)?;
    if let Some(a) = store.load_accounts()? {
        store.save_accounts(&a)?;
        println!("rewrote {}", ServerFile::Accounts.name());
    }
    if let Some(s) = store.load_settings()? {
        store.save_settings(&s)?;
        println!("rewrote {}", ServerFile::Settings.name());
    }
    Ok(())
}
