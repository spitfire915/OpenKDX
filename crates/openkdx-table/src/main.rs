#![forbid(unsafe_code)]
//! `openkdx-table`: fetch, import, and inspect the KDX challenge tables.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use openkdx_table::{Cache, Story, TableError};

#[derive(Parser)]
#[command(version, about = "Fetch, import, and inspect the KDX challenge tables")]
struct Cli {
    /// Cache directory (default: the per-user cache directory).
    #[arg(long, global = true)]
    cache: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum Which {
    Kdx,
    Hgc,
}

impl From<Which> for Story {
    fn from(w: Which) -> Self {
        match w {
            Which::Kdx => Story::Kdx,
            Which::Hgc => Story::Gconsole,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Download the smallest Haxial archive that holds each table and cache it.
    Fetch {
        /// Re-download even if the cache already holds a valid table.
        #[arg(long)]
        force: bool,
        /// Only this table (default: both).
        which: Option<Which>,
    },
    /// Take every table found in a KDX program, zip archive, or saved table.
    Import { file: PathBuf },
    /// Print the cache directory and which tables it holds.
    Path,
    /// Verify the cached tables.
    Verify,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cache = Cache::new(cli.cache.unwrap_or_else(Cache::default_dir));
    match run(cli.cmd, &cache) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("openkdx-table: {e}");
            ExitCode::from(exit_code(&e))
        }
    }
}

fn exit_code(e: &TableError) -> u8 {
    match e {
        TableError::AnchorNotFound | TableError::HashMismatch { .. } => 2,
        _ => 1,
    }
}

fn run(cmd: Cmd, cache: &Cache) -> Result<(), TableError> {
    match cmd {
        Cmd::Fetch { force, which } => {
            let stories: Vec<Story> = match which {
                Some(w) => vec![w.into()],
                None => Story::ALL.to_vec(),
            };
            let client = reqwest::blocking::Client::builder()
                .user_agent(concat!("openkdx-table/", env!("CARGO_PKG_VERSION")))
                .build()?;
            for story in stories {
                if !force && cache.get(story)?.is_some() {
                    println!("{}: already cached", story.file_name());
                    continue;
                }
                let bytes = openkdx_table::fetch(story, cache, &client)?;
                println!("{}: ok ({} bytes)", story.file_name(), bytes.len());
            }
            Ok(())
        }
        Cmd::Import { file } => {
            let found = openkdx_table::import(&file, cache)?;
            for s in found {
                println!("{}: imported ({} bytes)", s.file_name(), s.len());
            }
            Ok(())
        }
        Cmd::Path => {
            println!("{}", cache.dir().display());
            for story in Story::ALL {
                let state = match cache.get(story)? {
                    Some(_) => "present",
                    None => "missing",
                };
                println!("  {}: {state}", story.file_name());
            }
            Ok(())
        }
        Cmd::Verify => {
            let mut missing = false;
            for story in Story::ALL {
                match cache.get(story)? {
                    Some(_) => println!("{}: ok", story.file_name()),
                    None => {
                        println!("{}: missing or corrupt", story.file_name());
                        missing = true;
                    }
                }
            }
            if missing {
                Err(TableError::AnchorNotFound)
            } else {
                Ok(())
            }
        }
    }
}
