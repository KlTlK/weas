pub mod commands;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "weas",
    author = "KlTlK",
    version = "0.1.0",
    about = "Next-generation VPN client in Rust powered by sing-box"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the VPN client
    Run {
        /// Mode to run in (tun, sysproxy, mixed)
        #[arg(short, long, value_enum)]
        mode: Option<ModeArg>,

        /// Node ID or name to connect to
        #[arg(short, long)]
        node: Option<String>,
    },

    /// Manage subscriptions
    #[command(subcommand)]
    Sub(SubCommands),

    /// Manage proxy nodes
    #[command(subcommand)]
    Node(NodeCommands),

    /// Manage sing-box core binary
    #[command(subcommand)]
    Core(CoreCommands),

    /// Show current status and active node
    Status,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeArg {
    Tun,
    Sysproxy,
    Mixed,
}

#[derive(Subcommand, Debug)]
pub enum SubCommands {
    /// Add a new subscription URL
    Add {
        /// URL of the subscription (HTTP/HTTPS)
        url: String,

        /// Optional friendly name for subscription
        #[arg(short, long)]
        name: Option<String>,
    },

    /// List all configured subscriptions
    List,

    /// Update all subscriptions from remote sources
    Update,
}

#[derive(Subcommand, Debug)]
pub enum NodeCommands {
    /// List all nodes from subscriptions and custom entries
    List {
        /// Ping test nodes for latency
        #[arg(short, long)]
        ping: bool,
    },

    /// Select active node by ID or partial name
    Select {
        /// Node ID or partial name match
        target: String,
    },

    /// Add a direct proxy share link (vless://, ss://, trojan://)
    Add {
        /// Raw proxy link
        link: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CoreCommands {
    /// Check or auto-download sing-box binary
    Install,

    /// Show path to detected sing-box binary
    Path,
}
