use crate::config::app_config::{AppConfig, NodeInfo, ProxyMode};
use crate::gui::worker::{start_worker, GuiCommand, WorkerEvent};
use crate::parser::node::parse_node_link;
use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

#[derive(PartialEq, Eq)]
enum Tab {
    Servers,
    Subscriptions,
    AddNode,
    Settings,
}

#[derive(PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

pub struct WeasApp {
    config: AppConfig,
    state: ConnectionState,
    cmd_tx: Sender<GuiCommand>,
    event_rx: Receiver<WorkerEvent>,
    current_tab: Tab,
    search_query: String,
    sub_url_input: String,
    sub_name_input: String,
    node_link_input: String,
    status_message: Option<(String, bool)>, // (message, is_error)
    upload_speed: u64,
    download_speed: u64,
}

impl WeasApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup modern dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(20, 22, 28);
        visuals.panel_fill = Color32::from_rgb(24, 27, 34);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(28, 32, 40);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(34, 38, 48);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 48, 60);
        visuals.widgets.active.bg_fill = Color32::from_rgb(50, 58, 72);
        visuals.widgets.inactive.rounding = Rounding::same(6.0);
        visuals.widgets.hovered.rounding = Rounding::same(6.0);
        visuals.widgets.active.rounding = Rounding::same(6.0);
        cc.egui_ctx.set_visuals(visuals);

        let (cmd_tx, cmd_rx) = channel();
        let (event_tx, event_rx) = channel();

        start_worker(cmd_rx, event_tx);

        let config = AppConfig::load().unwrap_or_default();

        Self {
            config,
            state: ConnectionState::Disconnected,
            cmd_tx,
            event_rx,
            current_tab: Tab::Servers,
            search_query: String::new(),
            sub_url_input: String::new(),
            sub_name_input: String::new(),
            node_link_input: String::new(),
            status_message: None,
            upload_speed: 0,
            download_speed: 0,
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                WorkerEvent::Connecting => {
                    self.state = ConnectionState::Connecting;
                    self.status_message = Some(("Connecting to VPN...".into(), false));
                }
                WorkerEvent::Connected => {
                    self.state = ConnectionState::Connected;
                    self.status_message = Some(("VPN is connected and running!".into(), false));
                }
                WorkerEvent::Disconnected => {
                    self.state = ConnectionState::Disconnected;
                    self.upload_speed = 0;
                    self.download_speed = 0;
                    self.status_message = Some(("VPN disconnected.".into(), false));
                }
                WorkerEvent::Error(err) => {
                    self.status_message = Some((err, true));
                }
                WorkerEvent::Traffic { up, down } => {
                    self.upload_speed = up;
                    self.download_speed = down;
                }
                WorkerEvent::SubscriptionAdded(sub) => {
                    let count = sub.nodes.len();
                    if self.config.active_node_id.is_none() && !sub.nodes.is_empty() {
                        self.config.active_node_id = Some(sub.nodes[0].id.clone());
                    }
                    if let Some(pos) = self.config.subscriptions.iter().position(|s| s.url == sub.url) {
                        self.config.subscriptions[pos] = sub;
                    } else {
                        self.config.subscriptions.push(sub);
                    }
                    let _ = self.config.save();
                    self.sub_url_input.clear();
                    self.sub_name_input.clear();
                    self.status_message = Some((format!("Subscription added with {} nodes!", count), false));
                }
                WorkerEvent::SubscriptionsUpdated(subs) => {
                    self.config.subscriptions = subs;
                    let _ = self.config.save();
                    self.status_message = Some(("All subscriptions updated!".into(), false));
                }
            }
        }
    }

    fn format_speed(bytes_per_sec: u64) -> String {
        if bytes_per_sec < 1024 {
            format!("{} B/s", bytes_per_sec)
        } else if bytes_per_sec < 1024 * 1024 {
            format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
        } else {
            format!("{:.2} MB/s", bytes_per_sec as f64 / (1024.0 * 1024.0))
        }
    }
}

impl eframe::App for WeasApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();

        // Top Header
        egui::TopBottomPanel::top("header_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(20, 23, 30)).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("🛡️ WEAS VPN").size(22.0).bold().color(Color32::from_rgb(0, 220, 150)));
                    ui.label(RichText::new("v0.1.0 • sing-box core").size(12.0).color(Color32::from_rgb(140, 150, 170)));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_text = match self.config.mode {
                            ProxyMode::Tun => "Mode: TUN (Global VPN)",
                            ProxyMode::SysProxy => "Mode: System Proxy",
                            ProxyMode::MixedOnly => "Mode: SOCKS5 / HTTP (2080)",
                        };
                        ui.label(RichText::new(mode_text).size(13.0).bold().color(Color32::from_rgb(80, 180, 255)));
                    });
                });
            });

        // Hero Control Panel
        egui::TopBottomPanel::top("hero_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(26, 30, 40)).inner_margin(20.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Big Connect Button
                    let (btn_text, btn_color) = match self.state {
                        ConnectionState::Disconnected => ("⚡  CONNECT", Color32::from_rgb(45, 120, 240)),
                        ConnectionState::Connecting => ("⏳  CONNECTING...", Color32::from_rgb(220, 160, 40)),
                        ConnectionState::Connected => ("🟢  CONNECTED (DISCONNECT)", Color32::from_rgb(0, 190, 120)),
                    };

                    let btn = egui::Button::new(RichText::new(btn_text).size(18.0).bold().color(Color32::WHITE))
                        .min_size(Vec2::new(320.0, 54.0))
                        .fill(btn_color)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(10.0));

                    if ui.add(btn).clicked() {
                        match self.state {
                            ConnectionState::Disconnected => {
                                let _ = self.cmd_tx.send(GuiCommand::Connect);
                            }
                            ConnectionState::Connecting | ConnectionState::Connected => {
                                let _ = self.cmd_tx.send(GuiCommand::Disconnect);
                            }
                        }
                    }

                    ui.add_space(10.0);

                    // Active Node Info & Traffic
                    let active_node = self.config.get_active_node();
                    if let Some(node) = active_node {
                        ui.label(RichText::new(format!("Active Node: {} ({}:{})", node.name, node.server, node.port))
                            .size(14.0)
                            .color(Color32::from_rgb(220, 225, 235)));
                    } else {
                        ui.label(RichText::new("No active node selected. Please choose a node below.").size(14.0).color(Color32::from_rgb(240, 160, 60)));
                    }

                    if self.state == ConnectionState::Connected {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("↓ {}", Self::format_speed(self.download_speed))).size(14.0).bold().color(Color32::from_rgb(0, 210, 150)));
                            ui.label(RichText::new("  |  ").color(Color32::DARK_GRAY));
                            ui.label(RichText::new(format!("↑ {}", Self::format_speed(self.upload_speed))).size(14.0).bold().color(Color32::from_rgb(80, 180, 255)));
                        });
                    }

                    // Status / error banner
                    if let Some((ref msg, is_err)) = self.status_message {
                        ui.add_space(8.0);
                        let col = if is_err { Color32::from_rgb(255, 90, 90) } else { Color32::from_rgb(0, 210, 140) };
                        ui.label(RichText::new(msg).size(13.0).color(col));
                    }
                });
            });

        // Navigation Tabs
        egui::TopBottomPanel::top("tabs_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(22, 25, 32)).inner_margin(egui::Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let count = self.config.all_nodes().len();
                    ui.selectable_value(&mut self.current_tab, Tab::Servers, format!("🌐 Servers ({})", count));
                    ui.selectable_value(&mut self.current_tab, Tab::Subscriptions, format!("📡 Subscriptions ({})", self.config.subscriptions.len()));
                    ui.selectable_value(&mut self.current_tab, Tab::AddNode, "➕ Add Node");
                    ui.selectable_value(&mut self.current_tab, Tab::Settings, "⚙️ Settings");
                });
            });

        // Main Tab Content Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(18, 20, 26)).inner_margin(16.0))
            .show(ctx, |ui| {
                match self.current_tab {
                    Tab::Servers => self.show_servers_tab(ui),
                    Tab::Subscriptions => self.show_subscriptions_tab(ui),
                    Tab::AddNode => self.show_add_node_tab(ui),
                    Tab::Settings => self.show_settings_tab(ui),
                }
            });

        // Request repaint smoothly when connected for live speed stats
        if self.state == ConnectionState::Connected {
            ctx.request_repaint_after(Duration::from_millis(1000));
        }
    }
}

impl WeasApp {
    fn show_servers_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.add_space(8.0);

        let all_nodes = self.config.all_nodes();
        if all_nodes.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("No nodes found.").size(16.0).color(Color32::GRAY));
                ui.label("Go to 'Subscriptions' or 'Add Node' to add your proxy servers.");
            });
            return;
        }

        let active_id = self.config.active_node_id.clone().unwrap_or_default();
        let query = self.search_query.to_lowercase();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for node in all_nodes {
                if !query.is_empty() && !node.name.to_lowercase().contains(&query) && !node.server.contains(&query) {
                    continue;
                }

                let is_active = node.id == active_id;
                let bg_color = if is_active { Color32::from_rgb(32, 50, 42) } else { Color32::from_rgb(26, 30, 38) };

                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(12.0)
                    .stroke(Stroke::new(1.0, if is_active { Color32::from_rgb(0, 200, 130) } else { Color32::from_rgb(40, 44, 56) }))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut selected = is_active;
                            if ui.radio(&mut selected, "").clicked() || (!is_active && ui.button("Select").clicked()) {
                                self.config.active_node_id = Some(node.id.clone());
                                let _ = self.config.save();
                            }

                            ui.vertical(|ui| {
                                ui.label(RichText::new(&node.name).size(15.0).bold().color(Color32::WHITE));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(node.protocol.to_uppercase()).size(12.0).bold().color(Color32::from_rgb(255, 190, 60)));
                                    ui.label(RichText::new(format!("• {}:{}", node.server, node.port)).size(12.0).color(Color32::GRAY));
                                });
                            });
                        });
                    });

                ui.add_space(4.0);
            }
        });
    }

    fn show_subscriptions_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Manage Subscriptions");
        ui.add_space(8.0);

        egui::Frame::none()
            .fill(Color32::from_rgb(26, 30, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Add New Subscription").bold());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("URL:  ");
                    ui.add_sized(Vec2::new(450.0, 24.0), egui::TextEdit::singleline(&mut self.sub_url_input).hint_text("https://example.com/api/v1/client/subscribe?token=..."));
                });

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add_sized(Vec2::new(250.0, 24.0), egui::TextEdit::singleline(&mut self.sub_name_input).hint_text("Optional friendly name"));
                    
                    if ui.button(RichText::new("+ Add Subscription").bold()).clicked() {
                        let url = self.sub_url_input.trim().to_string();
                        if !url.is_empty() {
                            let name = if self.sub_name_input.trim().is_empty() { None } else { Some(self.sub_name_input.trim().to_string()) };
                            let _ = self.cmd_tx.send(GuiCommand::AddSubscription { url, name });
                        }
                    }
                });
            });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Configured Subscriptions").size(16.0).bold());
            if ui.button("🔄 Update All").clicked() {
                let _ = self.cmd_tx.send(GuiCommand::UpdateAllSubscriptions);
            }
        });

        ui.add_space(8.0);

        if self.config.subscriptions.is_empty() {
            ui.label("No subscriptions added yet.");
            return;
        }

        let mut to_remove = None;
        for (idx, sub) in self.config.subscriptions.iter().enumerate() {
            egui::Frame::none()
                .fill(Color32::from_rgb(24, 28, 36))
                .rounding(Rounding::same(8.0))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&sub.name).size(15.0).bold().color(Color32::from_rgb(0, 210, 150)));
                            ui.label(RichText::new(format!("{} nodes • {}", sub.nodes.len(), sub.url)).size(12.0).color(Color32::GRAY));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑️ Delete").clicked() {
                                to_remove = Some(idx);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }

        if let Some(idx) = to_remove {
            self.config.subscriptions.remove(idx);
            let _ = self.config.save();
        }
    }

    fn show_add_node_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Add Custom Proxy Node");
        ui.label("Paste a raw share link (vless://, ss://, or trojan://):");
        ui.add_space(8.0);

        ui.add_sized(Vec2::new(ui.available_width(), 100.0), egui::TextEdit::multiline(&mut self.node_link_input).hint_text("vless://uuid@server:443?security=reality&...#MyNode"));

        ui.add_space(8.0);

        if ui.button(RichText::new("Parse & Add Node").bold()).clicked() {
            let link = self.node_link_input.trim();
            match parse_node_link(link) {
                Ok(node) => {
                    let name = node.name.clone();
                    if self.config.active_node_id.is_none() {
                        self.config.active_node_id = Some(node.id.clone());
                    }
                    self.config.custom_nodes.push(node);
                    let _ = self.config.save();
                    self.node_link_input.clear();
                    self.status_message = Some((format!("Node '{}' added successfully!", name), false));
                    self.current_tab = Tab::Servers;
                }
                Err(e) => {
                    self.status_message = Some((format!("Invalid node link: {}", e), true));
                }
            }
        }
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("VPN & Client Settings");
        ui.add_space(12.0);

        egui::Frame::none()
            .fill(Color32::from_rgb(26, 30, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Operating Mode").bold());
                ui.radio_value(&mut self.config.mode, ProxyMode::Tun, "TUN Mode (Global VPN adapter, routes all system apps)");
                ui.radio_value(&mut self.config.mode, ProxyMode::SysProxy, "System Proxy (Sets Windows Proxy settings, lighter)");
                ui.radio_value(&mut self.config.mode, ProxyMode::MixedOnly, "Mixed Port Only (Manual proxy 127.0.0.1:2080)");

                ui.add_space(14.0);
                ui.label(RichText::new("Ports").bold());
                ui.horizontal(|ui| {
                    ui.label("Mixed Proxy Port: ");
                    ui.add(egui::DragValue::new(&mut self.config.mixed_port).range(1024..=65535));
                });
                ui.horizontal(|ui| {
                    ui.label("Clash API Port:   ");
                    ui.add(egui::DragValue::new(&mut self.config.clash_api_port).range(1024..=65535));
                });

                ui.add_space(14.0);
                if ui.button("Save Settings").clicked() {
                    let _ = self.config.save();
                    self.status_message = Some(("Settings saved successfully!".into(), false));
                }
            });
    }
}
