use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProxyMode {
    Tun,
    SysProxy,
    MixedOnly,
}

impl Default for ProxyMode {
    fn default() -> Self {
        Self::Tun
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub server: String,
    pub port: u16,
    pub raw_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub name: String,
    pub url: String,
    pub updated_at: u64,
    pub nodes: Vec<NodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode: ProxyMode,
    pub active_node_id: Option<String>,
    pub mixed_port: u16,
    pub clash_api_port: u16,
    pub subscriptions: Vec<Subscription>,
    pub custom_nodes: Vec<NodeInfo>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: ProxyMode::Tun,
            active_node_id: None,
            mixed_port: 2080,
            clash_api_port: 9090,
            subscriptions: Vec::new(),
            custom_nodes: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "weas", "weas")
            .context("Failed to resolve user configuration directory")?;
        let dir = proj_dirs.config_dir();
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        Ok(dir.to_path_buf())
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("weas_config.json"))
    }

    pub fn singbox_config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("singbox_generated.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }
        let content = fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn all_nodes(&self) -> Vec<NodeInfo> {
        let mut nodes = self.custom_nodes.clone();
        for sub in &self.subscriptions {
            nodes.extend(sub.nodes.clone());
        }
        nodes
    }

    pub fn get_active_node(&self) -> Option<NodeInfo> {
        let all = self.all_nodes();
        if let Some(ref id) = self.active_node_id {
            all.into_iter().find(|n| &n.id == id)
        } else {
            all.into_iter().next()
        }
    }
}
