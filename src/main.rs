use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use weas::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "weas=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Run { mode, node } => weas::cli::commands::run(mode, node).await,
            Commands::Sub(sub_cmd) => weas::cli::commands::handle_subscription(sub_cmd).await,
            Commands::Node(node_cmd) => weas::cli::commands::handle_node(node_cmd).await,
            Commands::Core(core_cmd) => weas::cli::commands::handle_core(core_cmd).await,
            Commands::Status => weas::cli::commands::status().await,
        },
        None => {
            // Default action if no subcommand is passed: start VPN with active node
            weas::cli::commands::run(None, None).await
        }
    }
}
