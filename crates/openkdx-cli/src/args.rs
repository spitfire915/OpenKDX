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
    /// Chat rooms.
    #[command(subcommand)]
    Chat(ChatCmd),
    /// Send a private message, or wait for them.
    #[command(subcommand)]
    Pm(PmCmd),
    /// Newsgroups and news messages.
    #[command(subcommand)]
    News(NewsCmd),
}

#[derive(Debug, Subcommand)]
pub enum NewsCmd {
    /// List the newsgroups.
    Groups,
    /// List the messages in a group (ID or name).
    List { group: String },
    /// Read one message.
    Read { group: String, message: u32 },
    /// Post a message.
    Post {
        group: String,
        #[arg(long)]
        subject: String,
        /// The body; lines may end in LF, CR, or CRLF.
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        /// Read the body from a file.
        #[arg(long)]
        body_file: Option<std::path::PathBuf>,
    },
    /// Delete a message.
    Delete { group: String, message: u32 },
    /// Create a newsgroup.
    CreateGroup {
        name: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Delete a newsgroup.
    DeleteGroup { group: String },
}

#[derive(Debug, Subcommand)]
pub enum ChatCmd {
    /// List the chat rooms.
    List,
    /// Join a chat and show its members and topic.
    Join { chat: String },
    /// Say something in a chat.
    Say {
        chat: String,
        #[arg(trailing_var_arg = true, required = true)]
        text: Vec<String>,
    },
    /// Send a `/me` action.
    Me {
        chat: String,
        #[arg(trailing_var_arg = true, required = true)]
        text: Vec<String>,
    },
    /// Set the topic.
    Topic {
        chat: String,
        #[arg(trailing_var_arg = true, required = true)]
        text: Vec<String>,
    },
    /// Join a chat and print what happens for a while.
    Watch {
        chat: String,
        /// Seconds to watch.
        #[arg(long, default_value = "30")]
        r#for: u64,
    },
    /// Leave a chat.
    Leave { chat: String },
}

#[derive(Debug, Subcommand)]
pub enum PmCmd {
    /// Send a private message to a user (ID or nick).
    Send {
        user: String,
        #[arg(trailing_var_arg = true, required = true)]
        text: Vec<String>,
    },
    /// Print incoming private messages for a while.
    Wait {
        #[arg(long, default_value = "30")]
        r#for: u64,
    },
}

/// One line of `run` input, parsed with the same subcommands.
#[derive(Debug, Parser)]
#[command(name = "", no_binary_name = true, disable_help_flag = true)]
pub struct RunLine {
    #[command(subcommand)]
    pub command: Command,
}
