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
    /// Files and folders.
    #[command(subcommand)]
    Files(FilesCmd),
    /// Accounts.
    #[command(subcommand)]
    Accounts(AccountsCmd),
    /// Account classes.
    #[command(subcommand)]
    Classes(ClassesCmd),
    /// Folder access items (drop boxes, upload folders, ...).
    #[command(subcommand)]
    FolderAccess(FaCmd),
    /// Server administration.
    #[command(subcommand)]
    Server(ServerCmd),
    /// Browse a tracker.
    Tracker {
        /// Tracker address, HOST or HOST:PORT (port 10800 by default).
        #[arg(short, long, env = "OPENKDX_TRACKER", default_value = "127.0.0.1")]
        tracker: String,
        #[command(subcommand)]
        command: TrackerCmd,
    },
}

#[derive(Debug, Subcommand)]
pub enum TrackerCmd {
    /// List the groups.
    Groups,
    /// List the servers in a group (the first group by default).
    Servers { group: Option<String> },
}

#[derive(Debug, Subcommand)]
pub enum ServerCmd {
    /// Show the server settings.
    Settings,
    /// Rename the server.
    SetName { name: String },
    /// Broadcast a message to every user.
    Broadcast {
        #[arg(trailing_var_arg = true, required = true)]
        text: Vec<String>,
    },
    /// Disconnect a user (ID or nick), optionally banning the address.
    Disconnect {
        user: String,
        /// Ban for this many seconds.
        #[arg(long, default_value = "0")]
        ban: u32,
        #[arg(long, default_value = "")]
        message: String,
        /// No notice to the other users.
        #[arg(long)]
        quiet: bool,
    },
    /// Show the server history, optionally following new records.
    History {
        /// Keep printing new records for this many seconds.
        #[arg(long)]
        follow: Option<u64>,
    },
    /// Show the connection monitor.
    Monitor {
        /// Poll for this many seconds.
        #[arg(long)]
        r#for: Option<u64>,
        /// Seconds between polls.
        #[arg(long, default_value = "2")]
        every: u64,
    },
    /// Shut the server down.
    Shutdown {
        #[arg(long, default_value = "0")]
        mode: u8,
        #[arg(long, default_value = "")]
        message: String,
    },
    /// Extra server information: uptime, OS, your address.
    Info,
}

#[derive(Debug, Subcommand)]
pub enum AccountsCmd {
    /// List the accounts.
    List,
    /// Show one account.
    Show {
        #[arg(value_name = "LOGIN")]
        account: String,
    },
    /// Create an account.
    Create {
        #[arg(value_name = "LOGIN")]
        account: String,
        /// The account's password (`--password` is your own login).
        #[arg(long, default_value = "")]
        new_password: String,
        #[arg(long, default_value = "")]
        name: String,
        #[arg(long, default_value = "1")]
        class: u8,
        /// `all`, `none`, or comma-separated names such as can-login,download.
        #[arg(long, default_value = "none")]
        privileges: String,
    },
    /// Change an account.
    Set {
        #[arg(value_name = "LOGIN")]
        account: String,
        #[arg(long)]
        new_password: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        class: Option<u8>,
        #[arg(long)]
        privileges: Option<String>,
    },
    /// Delete an account.
    Delete {
        #[arg(value_name = "LOGIN")]
        account: String,
    },
    /// Change your own password.
    Passwd { new_password: String },
}

#[derive(Debug, Subcommand)]
pub enum ClassesCmd {
    /// List the classes.
    List,
    /// Show one class.
    Show { index: u8 },
    /// Change a class.
    Set {
        index: u8,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        privileges: Option<String>,
        #[arg(long)]
        speed_limit: Option<u16>,
        /// relaxed, average, or strict
        #[arg(long)]
        flood: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum FaCmd {
    /// List the folder access items.
    List,
    /// Show one item.
    Show { name: String },
    /// Create an item.
    Create {
        name: String,
        /// normal, uploads, dropbox, pictures, or audio
        #[arg(long, default_value = "normal")]
        icon: String,
        #[arg(long, default_value = "")]
        owner: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Delete an item.
    Delete { name: String },
}

#[derive(Debug, Subcommand)]
pub enum FilesCmd {
    /// List a folder (the base folder by default).
    Ls {
        #[arg(default_value = "")]
        path: String,
        /// Sizes, dates, kinds.
        #[arg(short = 'L', long)]
        long: bool,
    },
    /// Information about a file or folder.
    Info { path: String },
    /// Create a folder.
    Mkdir {
        /// The parent folder ("" for the base).
        parent: String,
        name: String,
        /// A folder access item name, such as Uploads or a drop box.
        #[arg(long, default_value = "")]
        access: String,
        /// The drop box owner's login.
        #[arg(long, default_value = "")]
        owner: String,
    },
    /// Move items to the trash.
    Trash {
        #[arg(required = true)]
        paths: Vec<String>,
    },
    /// Empty the trash.
    EmptyTrash,
    /// Search the catalog.
    Search { text: String },
    /// Download a file.
    Get {
        path: String,
        /// Destination file or directory (the current directory by default).
        #[arg(long)]
        to: Option<std::path::PathBuf>,
        /// Continue a partial download.
        #[arg(long)]
        resume: bool,
    },
    /// Upload a file.
    Put {
        local: std::path::PathBuf,
        /// The destination folder ("" for the base).
        #[arg(default_value = "")]
        folder: String,
        /// The name on the server (the local name by default).
        #[arg(long)]
        name: Option<String>,
        /// Continue a partial upload.
        #[arg(long)]
        resume: bool,
        /// Test only: drop the connection after this many data bytes.
        #[arg(long, hide = true)]
        stop_after: Option<u64>,
    },
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
        /// Archive size limit in KiB (4..=1024).
        #[arg(long, default_value_t = 50)]
        size_kib: u16,
    },
    /// Change a newsgroup's archive size limit.
    SetGroupSize { group: String, size_kib: u16 },
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
