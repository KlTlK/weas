use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingBoxConfig {
    pub log: LogConfig,
    pub dns: DnsConfig,
    pub inbounds: Vec<Inbound>,
    pub outbounds: Vec<Outbound>,
    pub route: RouteConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub disabled: bool,
    pub level: String,
    pub timestamp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub servers: Vec<DnsServer>,
    pub rules: Vec<DnsRule>,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServer {
    pub tag: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clash_mode: Option<String>,
    pub server: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Inbound {
    #[serde(rename = "tun")]
    Tun {
        tag: String,
        inet4_address: String,
        auto_route: bool,
        strict_route: bool,
        stack: String,
        sniff: bool,
    },
    #[serde(rename = "mixed")]
    Mixed {
        tag: String,
        listen: String,
        listen_port: u16,
        sniff: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Outbound {
    #[serde(rename = "direct")]
    Direct { tag: String },
    #[serde(rename = "block")]
    Block { tag: String },
    #[serde(rename = "dns")]
    Dns { tag: String },
    #[serde(rename = "vless")]
    Vless {
        tag: String,
        server: String,
        server_port: u16,
        uuid: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tls: Option<TlsConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        transport: Option<TransportConfig>,
    },
    #[serde(rename = "shadowsocks")]
    Shadowsocks {
        tag: String,
        server: String,
        server_port: u16,
        method: String,
        password: String,
    },
    #[serde(rename = "trojan")]
    Trojan {
        tag: String,
        server: String,
        server_port: u16,
        password: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tls: Option<TlsConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    pub insecure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality: Option<RealityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityConfig {
    pub enabled: bool,
    pub public_key: String,
    pub short_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    #[serde(rename = "type")]
    pub transport_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub rules: Vec<RouteRule>,
    pub auto_detect_interface: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_is_private: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clash_mode: Option<String>,
    pub outbound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    pub clash_api: ClashApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashApiConfig {
    pub external_controller: String,
    #[serde(default)]
    pub external_ui: String,
    #[serde(default)]
    pub secret: String,
    pub default_mode: String,
}
