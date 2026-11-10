#![forbid(unsafe_code)]
//! `openkdx-dissect`: decode KDX connections in a packet capture.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use openkdx_dissect::audit::{Audit, audit};
use openkdx_dissect::connection::{Options, Ports, Record, dissect_capture};
use openkdx_dissect::text::{hexdump, summary};
use openkdx_proto::ChallengeTable;
use openkdx_table::{Cache, Story};

#[derive(Parser)]
#[command(version, about = "Decode KDX connections in a pcap or pcapng capture")]
struct Cli {
    /// pcap or pcapng file
    capture: PathBuf,
    /// Candidate account password; repeat for several. The empty password is always tried.
    #[arg(short, long = "password")]
    passwords: Vec<String>,
    /// The KDX challenge table, to verify the proofs' challenge bytes
    /// (default: OPENKDX_KDX_TABLE or the openkdx-table cache)
    #[arg(long)]
    table: Option<PathBuf>,
    /// Skip the challenge checks even if a table is available
    #[arg(long)]
    no_table: bool,
    /// After each message, say whether it re-encodes byte for byte
    #[arg(long)]
    audit: bool,
    /// Hexdump every payload
    #[arg(long)]
    hex: bool,
    /// Print only the `## Messages` inventory for the capture's notes
    #[arg(long)]
    inventory: bool,
    /// Write each connection's reassembled streams to DIR as
    /// `<client port>.c2s.bin` and `.s2c.bin`
    #[arg(long, value_name = "DIR")]
    export: Option<PathBuf>,
    #[arg(long, default_value_t = 10600)]
    dcc_port: u16,
    #[arg(long, default_value_t = 10700)]
    server_port: u16,
    #[arg(long, default_value_t = 10800)]
    tracker_port: u16,
    #[arg(long, default_value_t = 10900)]
    gconsole_port: u16,
}

fn load_table(cli: &Cli) -> anyhow::Result<Option<ChallengeTable>> {
    if cli.no_table {
        return Ok(None);
    }
    if let Some(path) = &cli.table {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Story::Kdx.verify(&bytes)?;
        return Ok(Some(ChallengeTable::new(bytes)));
    }
    if let Some(bytes) = openkdx_table::from_env(Story::Kdx)? {
        return Ok(Some(ChallengeTable::new(bytes)));
    }
    Ok(Cache::new(Cache::default_dir())
        .get(Story::Kdx)?
        .map(ChallengeTable::new))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(clean) => {
            if clean {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        Err(e) => {
            eprintln!("openkdx-dissect: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<bool> {
    let table = load_table(cli)?;
    if table.is_none() {
        eprintln!("note: no KDX challenge table; challenge bytes are not checked");
    }
    let opts = Options {
        ports: Ports {
            dcc: cli.dcc_port,
            server: cli.server_port,
            tracker: cli.tracker_port,
            gconsole: cli.gconsole_port,
        },
        passwords: cli
            .passwords
            .iter()
            .map(|p| p.as_bytes().to_vec())
            .collect(),
        table,
    };
    let cap = dissect_capture(&cli.capture, &opts)?;
    if cli.inventory {
        print!("{}", openkdx_dissect::text::inventory(&cap));
        return Ok(true);
    }
    let mut clean = true;

    for r in &cap.registrations {
        print!(
            "[{}] registration {} -> {} len {}",
            r.frame,
            r.src,
            r.dst,
            r.datagram.len()
        );
        match &r.registration {
            Ok(reg) => {
                println!(
                    "  port {} users {} name {} groups {}",
                    reg.server_port,
                    reg.users,
                    openkdx_dissect::text::quoted(&reg.name),
                    reg.groups
                        .iter()
                        .filter(|g| !g.is_empty())
                        .map(|g| openkdx_dissect::text::quoted(g))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if cli.audit {
                    let exact = reg.encode() == r.datagram;
                    println!("      audit: {}", if exact { "exact" } else { "MISMATCH" });
                    clean &= exact;
                }
            }
            Err(e) => {
                println!("  undecoded: {e}");
                clean = false;
            }
        }
        if cli.hex {
            print!("{}", hexdump(&r.datagram, "      "));
        }
    }

    for c in &cap.connections {
        println!("{} connection {} -> {}", c.kind, c.client, c.server);
        if let Some(k) = &c.key {
            println!("  key: {}", k.label);
        }
        for r in &c.records {
            println!("  [{}] {} {}", r.frame(), r.dir(), summary(r));
            match r {
                Record::Message {
                    message, payload, ..
                } => {
                    if cli.audit {
                        let a = audit(message, payload);
                        println!("      audit: {a}");
                        clean &= matches!(a, Audit::Exact | Audit::Unknown);
                    }
                    if cli.hex {
                        print!("{}", hexdump(payload, "      "));
                    }
                }
                Record::Stopped { .. } => clean = false,
                _ => {}
            }
        }
    }

    if let Some(dir) = &cli.export {
        let n = openkdx_dissect::connection::export_streams(&cli.capture, &opts.ports, dir)?;
        eprintln!("exported {n} connections to {}", dir.display());
    }
    Ok(clean)
}
