use crate::config::app_config::{NodeInfo, Subscription};
use crate::parser::node::parse_node_link;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn fetch_and_parse_subscription(url: &str, name: Option<&str>) -> Result<Subscription> {
    let client = reqwest::Client::builder()
        .user_agent("weas/0.1.0")
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch subscription. HTTP Status: {}",
            response.status()
        ));
    }

    let text = response.text().await?;
    let nodes = parse_subscription_content(&text)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let sub_name = name
        .map(|n| n.to_string())
        .unwrap_or_else(|| derive_sub_name(url));

    Ok(Subscription {
        name: sub_name,
        url: url.to_string(),
        updated_at: now,
        nodes,
    })
}

pub fn parse_subscription_content(content: &str) -> Result<Vec<NodeInfo>> {
    let content = content.trim();

    // Check if the content is Base64 encoded
    let raw_text = match STANDARD.decode(content.replace(['\r', '\n'], "").trim()) {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_else(|_| content.to_string()),
        Err(_) => content.to_string(),
    };

    let mut nodes = Vec::new();
    for line in raw_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(node) = parse_node_link(line) {
            nodes.push(node);
        }
    }

    if nodes.is_empty() {
        return Err(anyhow!("No valid proxy nodes found in subscription content"));
    }

    Ok(nodes)
}

fn derive_sub_name(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    "Default-Subscription".to_string()
}
