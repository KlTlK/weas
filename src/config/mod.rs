pub mod app_config;
pub mod generator;
pub mod singbox;

pub use app_config::{AppConfig, NodeInfo, ProxyMode, Subscription};
pub use generator::generate_singbox_config;
