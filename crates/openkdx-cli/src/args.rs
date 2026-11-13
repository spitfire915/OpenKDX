use std::time::Duration;

use clap::{Args, Parser, Subcommand};

/// A command-line KDX client.
#[derive(Debug, Parser)]
#[command(name = "openkdx", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub globals: Globals,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Args)]
pub struct Globals {
    /// Server address, HOST or HOST:PORT (port 10700 by default).
    #[arg(
        short,
        long,
        env = "OPENKDX_SERVER",
        default_value = "127.0.0.1",
        global = true
    )]
    pub server: String,
    /// Login name; empty for guest.
    #[arg(short, long, default_value = "", global = true)]
    pub login: String,
    /// Password; empty for guest.
    #[arg(
        short,
        long,
        env = "OPENKDX_PASSWORD",
        default_value = "",
        global = true
    )]
    pub password: String,
    /// The nick other users see.
    #[arg(long, default_value = "openkdx", global = true)]
    pub nick: String,
    /// One JSON object per line instead of text.
    #[arg(long, global = true)]
    pub json: bool,
    /// Seconds to wait for the server.
    #[arg(long, default_value = "30", global = true)]
    pub timeout: u64,
    /// Print less.
    #[arg(short, long, global = true)]
    pub quiet: bool,
    /// Test only: use the built-in placeholder challenge table.
    #[arg(long, hide = true, global = true)]
    pub insecure_default_table: bool,
}

impl Globals {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout)
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Log in and show the server's information.
    Login {
        /// Stay connected this many seconds and print what happens.
        #[arg(long)]
        stay: Option<u64>,
    },
    /// List the users who are logged in.
    Users,
    /// Read commands from stdin, one per line, on a single session.
    Run {
        /// Keep going after a command fails.
        #[arg(long)]
        keep_going: bool,
    },
}

/// One line of `run` input, parsed with the same subcommands.
#[derive(Debug, Parser)]
#[command(name = "", no_binary_name = true, disable_help_flag = true)]
pub struct RunLine {
    #[command(subcommand)]
    pub command: Command,
}
