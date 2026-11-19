use crate::config::app_config::{AppConfig, ProxyMode, Subscription};
use crate::config::generator::generate_singbox_config;
use crate::core::api::ClashApiClient;
use crate::core::binary::ensure_singbox_binary;
use crate::core::process::SingBoxProcess;
use crate::parser::subscription::fetch_and_parse_subscription;
use crate::sys::SystemProxy;
use std::fs;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use tokio::time::sleep;

pub enum GuiCommand {
    Connect,
    Disconnect,
    AddSubscription { url: String, name: Option<String> },
    UpdateAllSubscriptions,
}

pub enum WorkerEvent {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
    Traffic { up: u64, down: u64 },
    SubscriptionAdded(Subscription),
    SubscriptionsUpdated(Vec<Subscription>),
}

pub fn start_worker(cmd_rx: Receiver<GuiCommand>, event_tx: Sender<WorkerEvent>) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                let _ = event_tx.send(WorkerEvent::Error(format!("Runtime init failed: {}", e)));
                return;
            }
        };

        rt.block_on(worker_loop(cmd_rx, event_tx));
    });
}

async fn worker_loop(cmd_rx: Receiver<GuiCommand>, event_tx: Sender<WorkerEvent>) {
    let mut current_process: Option<SingBoxProcess> = None;
    let mut is_connected = false;
    let mut api_port: u16 = 9090;
    let mut active_mode = ProxyMode::Tun;
    let mut mixed_port: u16 = 2080;

    loop {
        // 1. Process pending GUI commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                GuiCommand::Connect => {
                    let config = match AppConfig::load() {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Config error: {}", e)));
                            continue;
                        }
                    };

                    let active_node = match config.get_active_node() {
                        Some(n) => n,
                        None => {
                            let _ = event_tx.send(WorkerEvent::Error("No node selected! Add a subscription or node first.".into()));
                            continue;
                        }
                    };

                    let _ = event_tx.send(WorkerEvent::Connecting);

                    api_port = config.clash_api_port;
                    active_mode = config.mode.clone();
                    mixed_port = config.mixed_port;

                    let bin_path = match ensure_singbox_binary().await {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Failed to obtain sing-box core: {}", e)));
                            let _ = event_tx.send(WorkerEvent::Disconnected);
                            continue;
                        }
                    };

                    let singbox_cfg = match generate_singbox_config(&config, &active_node) {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Failed to build configuration: {}", e)));
                            let _ = event_tx.send(WorkerEvent::Disconnected);
                            continue;
                        }
                    };

                    let cfg_path = match AppConfig::singbox_config_path() {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Config path error: {}", e)));
                            let _ = event_tx.send(WorkerEvent::Disconnected);
                            continue;
                        }
                    };

                    let json = match serde_json::to_string_pretty(&singbox_cfg) {
                        Ok(j) => j,
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("JSON serialization error: {}", e)));
                            let _ = event_tx.send(WorkerEvent::Disconnected);
                            continue;
                        }
                    };

                    if let Err(e) = fs::write(&cfg_path, json) {
                        let _ = event_tx.send(WorkerEvent::Error(format!("Failed to write config: {}", e)));
                        let _ = event_tx.send(WorkerEvent::Disconnected);
                        continue;
                    }

                    if active_mode == ProxyMode::SysProxy {
                        if let Err(e) = SystemProxy::enable(mixed_port) {
                            let _ = event_tx.send(WorkerEvent::Error(format!("SysProxy error: {}", e)));
                        }
                    }

                    match SingBoxProcess::spawn(&bin_path, &cfg_path) {
                        Ok(proc) => {
                            current_process = Some(proc);
                            is_connected = true;
                            let _ = event_tx.send(WorkerEvent::Connected);
                        }
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Failed to start core process: {}", e)));
                            let _ = event_tx.send(WorkerEvent::Disconnected);
                        }
                    }
                }
                GuiCommand::Disconnect => {
                    if let Some(mut proc) = current_process.take() {
                        let _ = proc.stop().await;
                    }
                    if active_mode == ProxyMode::SysProxy {
                        let _ = SystemProxy::disable();
                    }
                    is_connected = false;
                    let _ = event_tx.send(WorkerEvent::Disconnected);
                }
                GuiCommand::AddSubscription { url, name } => {
                    match fetch_and_parse_subscription(&url, name.as_deref()).await {
                        Ok(sub) => {
                            let _ = event_tx.send(WorkerEvent::SubscriptionAdded(sub));
                        }
                        Err(e) => {
                            let _ = event_tx.send(WorkerEvent::Error(format!("Failed to add subscription: {}", e)));
                        }
                    }
                }
                GuiCommand::UpdateAllSubscriptions => {
                    if let Ok(config) = AppConfig::load() {
                        let mut updated = Vec::new();
                        for sub in config.subscriptions {
                            if let Ok(new_sub) = fetch_and_parse_subscription(&sub.url, Some(&sub.name)).await {
                                updated.push(new_sub);
                            } else {
                                updated.push(sub);
                            }
                        }
                        let _ = event_tx.send(WorkerEvent::SubscriptionsUpdated(updated));
                    }
                }
            }
        }

        // 2. Poll traffic if active
        if is_connected {
            let api = ClashApiClient::new(api_port);
            if let Ok(traffic) = api.get_traffic().await {
                let _ = event_tx.send(WorkerEvent::Traffic {
                    up: traffic.up,
                    down: traffic.down,
                });
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
