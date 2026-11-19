use crate::cli::{CoreCommands, ModeArg, NodeCommands, SubCommands};
use crate::config::app_config::{AppConfig, ProxyMode};
use crate::config::generator::generate_singbox_config;
use crate::core::binary::{ensure_singbox_binary, find_singbox_binary};
use crate::core::process::SingBoxProcess;
use crate::parser::node::parse_node_link;
use crate::parser::subscription::fetch_and_parse_subscription;
use crate::sys::{is_admin, SystemProxy};
use anyhow::{bail, Context, Result};
use colored::*;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

pub async fn run(mode_override: Option<ModeArg>, node_override: Option<String>) -> Result<()> {
    let mut config = AppConfig::load()?;

    if let Some(m) = mode_override {
        config.mode = match m {
            ModeArg::Tun => ProxyMode::Tun,
            ModeArg::Sysproxy => ProxyMode::SysProxy,
            ModeArg::Mixed => ProxyMode::MixedOnly,
        };
    }

    if let Some(node_target) = node_override {
        let all = config.all_nodes();
        if let Some(found) = all.iter().find(|n| n.id == node_target || n.name.contains(&node_target)) {
            config.active_node_id = Some(found.id.clone());
        } else {
            bail!("Specified node '{}' not found", node_target);
        }
    }

    let active_node = config
        .get_active_node()
        .ok_or_else(|| anyhow::anyhow!("No active proxy node configured. Use 'weas sub add <URL>' first."))?;

    println!("{}", "==================================================".cyan());
    println!("  {}", "WEAS VPN CLIENT (powered by sing-box)".bold().green());
    println!("{}", "==================================================".cyan());
    println!("  • Node:      {}", active_node.name.bold());
    println!("  • Protocol:  {}", active_node.protocol.to_uppercase().yellow());
    println!("  • Server:    {}:{}", active_node.server, active_node.port);
    println!("  • Mode:      {:?}", config.mode);
    println!("  • Inbound:   127.0.0.1:{}", config.mixed_port);
    println!("{}", "==================================================".cyan());

    // Privilege check for TUN
    if config.mode == ProxyMode::Tun && !is_admin() {
        println!("{}", "WARNING: TUN mode typically requires Administrator/root privileges!".bold().yellow());
        println!("If network routing fails, please run the terminal as Administrator.\n");
    }

    // Ensure sing-box binary is ready
    let bin_path = ensure_singbox_binary().await?;

    // Generate sing-box config
    let singbox_cfg = generate_singbox_config(&config, &active_node)?;
    let singbox_cfg_path = AppConfig::singbox_config_path()?;
    let json_content = serde_json::to_string_pretty(&singbox_cfg)?;
    fs::write(&singbox_cfg_path, json_content)?;

    // Enable system proxy if in SysProxy mode
    if config.mode == ProxyMode::SysProxy {
        SystemProxy::enable(config.mixed_port)?;
    }

    // Set up graceful cleanup on Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let mode_clone = config.mode.clone();

    ctrlc::set_handler(move || {
        println!("\n{}", "Interrupt received, stopping weas VPN...".yellow());
        r.store(false, Ordering::SeqCst);
        if mode_clone == ProxyMode::SysProxy {
            let _ = SystemProxy::disable();
        }
    })?;

    // Start sing-box
    let mut process = SingBoxProcess::spawn(&bin_path, &singbox_cfg_path)?;
    println!("{}", "VPN is RUNNING. Press Ctrl+C to stop.".bold().green());

    tokio::select! {
        res = process.wait() => {
            if let Ok(status) = res {
                info!("sing-box process exited with status: {}", status);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n{}", "Shutting down...".yellow());
            let _ = process.stop().await;
        }
    }

    // Cleanup sysproxy
    if config.mode == ProxyMode::SysProxy {
        SystemProxy::disable()?;
    }

    println!("{}", "VPN connection stopped cleanly.".green());
    Ok(())
}

pub async fn handle_subscription(sub_cmd: SubCommands) -> Result<()> {
    let mut config = AppConfig::load()?;

    match sub_cmd {
        SubCommands::Add { url, name } => {
            println!("Fetching subscription from: {}", url.cyan());
            let sub = fetch_and_parse_subscription(&url, name.as_deref()).await?;
            println!(
                "{} Loaded subscription '{}' with {} nodes.",
                "SUCCESS:".bold().green(),
                sub.name.bold(),
                sub.nodes.len().to_string().yellow()
            );

            // Set first node as active if none set
            if config.active_node_id.is_none() && !sub.nodes.is_empty() {
                config.active_node_id = Some(sub.nodes[0].id.clone());
            }

            // Replace existing if same url or append
            if let Some(pos) = config.subscriptions.iter().position(|s| s.url == sub.url) {
                config.subscriptions[pos] = sub;
            } else {
                config.subscriptions.push(sub);
            }
            config.save()?;
            println!("Subscription saved to configuration.");
        }
        SubCommands::List => {
            if config.subscriptions.is_empty() {
                println!("No subscriptions configured yet. Add one with: weas sub add <URL>");
                return Ok(());
            }
            println!("{}", "Configured Subscriptions:".bold());
            for (idx, sub) in config.subscriptions.iter().enumerate() {
                println!(
                    "  [{}] {} ({} nodes) - URL: {}",
                    idx + 1,
                    sub.name.bold().green(),
                    sub.nodes.len(),
                    sub.url.dimmed()
                );
            }
        }
        SubCommands::Update => {
            println!("Updating all subscriptions...");
            let mut updated_subs = Vec::new();
            for sub in config.subscriptions {
                print!("Updating '{}' ... ", sub.name);
                match fetch_and_parse_subscription(&sub.url, Some(&sub.name)).await {
                    Ok(new_sub) => {
                        println!("{} ({} nodes)", "OK".green(), new_sub.nodes.len());
                        updated_subs.push(new_sub);
                    }
                    Err(e) => {
                        println!("{} ({})", "FAILED".red(), e);
                        updated_subs.push(sub);
                    }
                }
            }
            config.subscriptions = updated_subs;
            config.save()?;
            println!("All subscriptions updated.");
        }
    }
    Ok(())
}

pub async fn handle_node(node_cmd: NodeCommands) -> Result<()> {
    let mut config = AppConfig::load()?;

    match node_cmd {
        NodeCommands::List { ping: _ } => {
            let all = config.all_nodes();
            if all.is_empty() {
                println!("No nodes available. Add a subscription with: weas sub add <URL>");
                return Ok(());
            }

            println!("{}", "Available Nodes:".bold());
            let active_id = config.active_node_id.clone().unwrap_or_default();

            for (idx, node) in all.iter().enumerate() {
                let is_active = node.id == active_id;
                let marker = if is_active {
                    "[* ACTIVE]".bold().green()
                } else {
                    "          ".normal()
                };
                println!(
                    "{} [{:02}] {:<30} [{}] {}:{}",
                    marker,
                    idx + 1,
                    node.name.cyan(),
                    node.protocol.to_uppercase().yellow(),
                    node.server,
                    node.port
                );
            }
        }
        NodeCommands::Select { target } => {
            let all = config.all_nodes();
            let matched = if let Ok(idx) = target.parse::<usize>() {
                if idx > 0 && idx <= all.len() {
                    Some(all[idx - 1].clone())
                } else {
                    None
                }
            } else {
                all.into_iter()
                    .find(|n| n.id == target || n.name.to_lowercase().contains(&target.to_lowercase()))
            };

            if let Some(node) = matched {
                config.active_node_id = Some(node.id.clone());
                config.save()?;
                println!("Active node set to: {} ({})", node.name.green().bold(), node.id.dimmed());
            } else {
                bail!("Could not find node matching '{}'", target);
            }
        }
        NodeCommands::Add { link } => {
            let node = parse_node_link(&link)?;
            println!("Parsed custom node: {} [{}]", node.name.green(), node.protocol);
            if config.active_node_id.is_none() {
                config.active_node_id = Some(node.id.clone());
            }
            config.custom_nodes.push(node);
            config.save()?;
            println!("Custom node added and saved.");
        }
    }
    Ok(())
}

pub async fn handle_core(core_cmd: CoreCommands) -> Result<()> {
    match core_cmd {
        CoreCommands::Install => {
            let path = ensure_singbox_binary().await?;
            println!("sing-box core is ready at: {}", path.display().to_string().green());
        }
        CoreCommands::Path => {
            if let Some(path) = find_singbox_binary() {
                println!("sing-box detected at: {}", path.display().to_string().green());
            } else {
                println!("{}", "sing-box binary not found locally. Run 'weas core install' to download it.".yellow());
            }
        }
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    let config = AppConfig::load()?;
    let active_node = config.get_active_node();

    println!("{}", "WEAS Client Status".bold());
    println!("  • Config path: {}", AppConfig::config_path()?.display());
    println!("  • Mode:        {:?}", config.mode);
    println!("  • Mixed port:  {}", config.mixed_port);
    println!("  • Core binary: {:?}", find_singbox_binary().unwrap_or_else(|| "Not found".into()));

    if let Some(node) = active_node {
        println!("  • Active node: {} ({}:{})", node.name.green().bold(), node.server, node.port);
    } else {
        println!("  • Active node: {}", "None selected".yellow());
    }

    Ok(())
}
