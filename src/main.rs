use clap::Parser;
use colored::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use weas::cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "weas=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Some(cmd) => match cmd {
            Commands::Run { mode, node } => weas::cli::commands::run(mode, node).await,
            Commands::Sub(sub_cmd) => weas::cli::commands::handle_subscription(sub_cmd).await,
            Commands::Node(node_cmd) => weas::cli::commands::handle_node(node_cmd).await,
            Commands::Core(core_cmd) => weas::cli::commands::handle_core(core_cmd).await,
            Commands::Status => weas::cli::commands::status().await,
        },
        None => {
            show_welcome();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("\n{} {:#}", "ERROR:".bold().red(), e);
        wait_for_keypress();
        std::process::exit(1);
    }
}

fn show_welcome() {
    println!();
    println!("{}", "==================================================".cyan());
    println!("  {}", "WEAS VPN CLIENT v0.1.0".bold().green());
    println!("  {}", "Powered by sing-box".dimmed());
    println!("{}", "==================================================".cyan());
    println!();
    println!("  {}", "Quick Start:".bold().yellow());
    println!("  1. Add a subscription:");
    println!("     {} weas sub add \"https://your-subscription-url\"", "$".dimmed());
    println!();
    println!("  2. Or add a direct node link:");
    println!("     {} weas node add \"vless://...\"  or  \"ss://...\"  or  \"trojan://...\"", "$".dimmed());
    println!();
    println!("  3. Connect:");
    println!("     {} weas run --mode tun       {}", "$".dimmed(), "(full system VPN, needs Admin)".dimmed());
    println!("     {} weas run --mode sysproxy  {}", "$".dimmed(), "(system proxy, no Admin needed)".dimmed());
    println!();
    println!("  {}", "Other Commands:".bold().yellow());
    println!("     {} weas status               {}", "$".dimmed(), "Show current config and active node".dimmed());
    println!("     {} weas sub list              {}", "$".dimmed(), "List subscriptions".dimmed());
    println!("     {} weas sub update            {}", "$".dimmed(), "Update all subscriptions".dimmed());
    println!("     {} weas node list             {}", "$".dimmed(), "List all available nodes".dimmed());
    println!("     {} weas node select <id>      {}", "$".dimmed(), "Select active node".dimmed());
    println!("     {} weas core install          {}", "$".dimmed(), "Download sing-box core".dimmed());
    println!("     {} weas --help                {}", "$".dimmed(), "Full help".dimmed());
    println!();
    println!("{}", "==================================================".cyan());
    println!();

    // Show current status inline
    if let Ok(config) = weas::config::AppConfig::load() {
        let node_count = config.all_nodes().len();
        let active = config.get_active_node();
        println!("  {} {} subscription(s), {} node(s) configured",
            "Status:".bold(),
            config.subscriptions.len(),
            node_count,
        );
        if let Some(n) = active {
            println!("  {} {} [{}] {}:{}",
                "Active:".bold().green(),
                n.name, n.protocol.to_uppercase(), n.server, n.port
            );
        } else {
            println!("  {} {}", "Active:".bold(), "No node selected".yellow());
        }
        println!();
    }

    wait_for_keypress();
}

fn wait_for_keypress() {
    use std::io::{self, Read};
    println!("{}", "Press Enter to exit...".dimmed());
    let _ = io::stdin().read(&mut [0u8]);
}
