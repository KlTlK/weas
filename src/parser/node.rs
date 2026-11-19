use crate::config::app_config::NodeInfo;
use crate::config::singbox::{Outbound, RealityConfig, TlsConfig, TransportConfig};
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::HashMap;
use url::Url;

pub fn parse_node_link(link: &str) -> Result<NodeInfo> {
    let link = link.trim();
    if link.starts_with("vless://") {
        parse_vless_link(link)
    } else if link.starts_with("ss://") {
        parse_ss_link(link)
    } else if link.starts_with("trojan://") {
        parse_trojan_link(link)
    } else {
        bail!("Unsupported protocol link: {}", link);
    }
}

fn parse_vless_link(link: &str) -> Result<NodeInfo> {
    let parsed = Url::parse(link)?;
    let uuid = parsed.username();
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Missing host in VLESS link"))?;
    let port = parsed
        .port()
        .ok_or_else(|| anyhow!("Missing port in VLESS link"))?;
    let name = parsed
        .fragment()
        .map(|f| urlencoding_decode(f))
        .unwrap_or_else(|| format!("{}:{}", host, port));

    let id = format!("{:x}", md5_hash(link));

    Ok(NodeInfo {
        id,
        name,
        protocol: "vless".into(),
        server: host.into(),
        port,
        raw_link: link.into(),
    })
}

fn parse_ss_link(link: &str) -> Result<NodeInfo> {
    let stripped = link.trim_start_matches("ss://");
    let (body, fragment) = match stripped.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (stripped, None),
    };

    let name = fragment
        .map(|f| urlencoding_decode(f))
        .unwrap_or_else(|| "Shadowsocks".into());

    let (host, port) = if let Some((_, host_part)) = body.split_once('@') {
        let (h, p) = host_part
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid shadowsocks host:port"))?;
        (h.to_string(), p.parse::<u16>()?)
    } else {
        // May be entirely base64 encoded
        let decoded = decode_base64_loose(body)?;
        let decoded_str = String::from_utf8(decoded)?;
        let (credentials, host_part) = decoded_str
            .split_once('@')
            .ok_or_else(|| anyhow!("Invalid base64 shadowsocks body"))?;
        let (h, p) = host_part
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid shadowsocks host:port"))?;
        (h.to_string(), p.parse::<u16>()?)
    };

    let id = format!("{:x}", md5_hash(link));

    Ok(NodeInfo {
        id,
        name,
        protocol: "shadowsocks".into(),
        server: host,
        port,
        raw_link: link.into(),
    })
}

fn parse_trojan_link(link: &str) -> Result<NodeInfo> {
    let parsed = Url::parse(link)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Missing host in Trojan link"))?;
    let port = parsed
        .port()
        .ok_or_else(|| anyhow!("Missing port in Trojan link"))?;
    let name = parsed
        .fragment()
        .map(|f| urlencoding_decode(f))
        .unwrap_or_else(|| format!("Trojan-{}:{}", host, port));

    let id = format!("{:x}", md5_hash(link));

    Ok(NodeInfo {
        id,
        name,
        protocol: "trojan".into(),
        server: host.into(),
        port,
        raw_link: link.into(),
    })
}

pub fn parse_node_to_outbound(node: &NodeInfo) -> Result<Outbound> {
    let url = Url::parse(&node.raw_link)?;
    let query: HashMap<String, String> = url.query_pairs().into_owned().collect();

    match node.protocol.as_str() {
        "vless" => {
            let uuid = url.username().to_string();
            let flow = query.get("flow").cloned();
            let security = query.get("security").map(|s| s.as_str()).unwrap_or("none");

            let tls = if security == "tls" || security == "reality" {
                let sni = query.get("sni").cloned().or_else(|| Some(node.server.clone()));
                let reality = if security == "reality" {
                    Some(RealityConfig {
                        enabled: true,
                        public_key: query.get("pbk").cloned().unwrap_or_default(),
                        short_id: query.get("sid").cloned().unwrap_or_default(),
                    })
                } else {
                    None
                };

                Some(TlsConfig {
                    enabled: true,
                    server_name: sni,
                    insecure: query.get("allowInsecure").map(|v| v == "1").unwrap_or(false),
                    reality,
                })
            } else {
                None
            };

            let transport_type = query.get("type").map(|s| s.as_str()).unwrap_or("tcp");
            let transport = if transport_type == "ws" {
                Some(TransportConfig {
                    transport_type: "ws".into(),
                    path: query.get("path").cloned(),
                    host: query.get("host").cloned(),
                })
            } else {
                None
            };

            Ok(Outbound::Vless {
                tag: "proxy".into(),
                server: node.server.clone(),
                server_port: node.port,
                uuid,
                flow,
                tls,
                transport,
            })
        }
        "shadowsocks" => {
            let stripped = node.raw_link.trim_start_matches("ss://");
            let body = stripped.split('#').next().unwrap_or(stripped);

            let (method, password) = if let Some((cred_part, _)) = body.split_once('@') {
                let decoded = decode_base64_loose(cred_part)?;
                let s = String::from_utf8(decoded)?;
                let (m, p) = s.split_once(':').ok_or_else(|| anyhow!("Invalid ss creds"))?;
                (m.to_string(), p.to_string())
            } else {
                let decoded = decode_base64_loose(body)?;
                let s = String::from_utf8(decoded)?;
                let (cred_part, _) = s.split_once('@').ok_or_else(|| anyhow!("Invalid ss"))?;
                let (m, p) = cred_part.split_once(':').ok_or_else(|| anyhow!("Invalid ss creds"))?;
                (m.to_string(), p.to_string())
            };

            Ok(Outbound::Shadowsocks {
                tag: "proxy".into(),
                server: node.server.clone(),
                server_port: node.port,
                method,
                password,
            })
        }
        "trojan" => {
            let password = url.username().to_string();
            let sni = query.get("sni").cloned().or_else(|| Some(node.server.clone()));

            Ok(Outbound::Trojan {
                tag: "proxy".into(),
                server: node.server.clone(),
                server_port: node.port,
                password,
                tls: Some(TlsConfig {
                    enabled: true,
                    server_name: sni,
                    insecure: query.get("allowInsecure").map(|v| v == "1").unwrap_or(false),
                    reality: None,
                }),
            })
        }
        _ => bail!("Unsupported outbound protocol: {}", node.protocol),
    }
}

fn decode_base64_loose(input: &str) -> Result<Vec<u8>> {
    let clean = input.replace('-', "+").replace('_', "/").trim().to_string();
    let pad_len = (4 - (clean.len() % 4)) % 4;
    let padded = format!("{}{}", clean, "=".repeat(pad_len));
    STANDARD
        .decode(&padded)
        .map_err(|e| anyhow!("Base64 decode failed: {}", e))
}

fn urlencoding_decode(s: &str) -> String {
    url::form_urlencoded::parse(s.as_bytes())
        .map(|(k, _)| k.into_owned())
        .collect::<Vec<_>>()
        .join("")
}

fn md5_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
