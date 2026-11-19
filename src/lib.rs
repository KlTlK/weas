pub mod cli;
pub mod config;
pub mod core;
pub mod gui;
pub mod parser;
pub mod sys;

pub async fn run_cli() {
    use clap::Parser;
    use cli::{Cli, Commands};

    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Run { mode, node } => {
                if let Err(e) = cli::commands::run(mode, node).await {
                    eprintln!("Error: {:#}", e);
                }
            }
            Commands::Sub(sub_cmd) => {
                if let Err(e) = cli::commands::handle_subscription(sub_cmd).await {
                    eprintln!("Error: {:#}", e);
                }
            }
            Commands::Node(node_cmd) => {
                if let Err(e) = cli::commands::handle_node(node_cmd).await {
                    eprintln!("Error: {:#}", e);
                }
            }
            Commands::Core(core_cmd) => {
                if let Err(e) = cli::commands::handle_core(core_cmd).await {
                    eprintln!("Error: {:#}", e);
                }
            }
            Commands::Status => {
                if let Err(e) = cli::commands::status().await {
                    eprintln!("Error: {:#}", e);
                }
            }
        },
        None => {
            if let Err(e) = cli::commands::run(None, None).await {
                eprintln!("Error: {:#}", e);
            }
        }
    }
}
