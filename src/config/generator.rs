use crate::config::app_config::{AppConfig, NodeInfo, ProxyMode};
use crate::config::singbox::*;
use crate::parser::node::parse_node_to_outbound;
use anyhow::Result;

pub fn generate_singbox_config(app_config: &AppConfig, node: &NodeInfo) -> Result<SingBoxConfig> {
    let mut inbounds = Vec::new();

    // 1. Mixed HTTP/SOCKS5 inbound (always useful for local apps or fallback)
    inbounds.push(Inbound::Mixed {
        tag: "mixed-in".into(),
        listen: "127.0.0.1".into(),
        listen_port: app_config.mixed_port,
        sniff: true,
    });

    // 2. TUN inbound if TUN mode is enabled
    if app_config.mode == ProxyMode::Tun {
        inbounds.push(Inbound::Tun {
            tag: "tun-in".into(),
            inet4_address: "172.19.0.1/30".into(),
            auto_route: true,
            strict_route: true,
            stack: "mixed".into(),
            sniff: true,
        });
    }

    // 3. Outbounds: proxy outbound from node
    let proxy_outbound = parse_node_to_outbound(node)?;

    let outbounds = vec![
        proxy_outbound,
        Outbound::Direct {
            tag: "direct".into(),
        },
        Outbound::Block {
            tag: "block".into(),
        },
        Outbound::Dns { tag: "dns-out".into() },
    ];

    // 4. DNS Setup
    let dns = DnsConfig {
        servers: vec![
            DnsServer {
                tag: "dns-remote".into(),
                address: "tls://1.1.1.1".into(),
                detour: Some("proxy".into()),
            },
            DnsServer {
                tag: "dns-direct".into(),
                address: "local".into(),
                detour: Some("direct".into()),
            },
            DnsServer {
                tag: "dns-block".into(),
                address: "rcode://success".into(),
                detour: None,
            },
        ],
        rules: vec![
            DnsRule {
                outbound: Some(vec!["any".into()]),
                clash_mode: None,
                server: "dns-direct".into(),
            },
            DnsRule {
                outbound: None,
                clash_mode: Some("Direct".into()),
                server: "dns-direct".into(),
            },
            DnsRule {
                outbound: None,
                clash_mode: Some("Global".into()),
                server: "dns-remote".into(),
            },
        ],
        strategy: "prefer_ipv4".into(),
    };

    // 5. Routing
    let route = RouteConfig {
        rules: vec![
            RouteRule {
                protocol: Some(vec!["dns".into()]),
                ip_is_private: None,
                clash_mode: None,
                outbound: "dns-out".into(),
            },
            RouteRule {
                protocol: None,
                ip_is_private: Some(true),
                clash_mode: None,
                outbound: "direct".into(),
            },
            RouteRule {
                protocol: None,
                ip_is_private: None,
                clash_mode: Some("Direct".into()),
                outbound: "direct".into(),
            },
            RouteRule {
                protocol: None,
                ip_is_private: None,
                clash_mode: Some("Global".into()),
                outbound: "proxy".into(),
            },
        ],
        auto_detect_interface: true,
    };

    // 6. Clash API (for live stats and speed/latency)
    let experimental = Some(ExperimentalConfig {
        clash_api: ClashApiConfig {
            external_controller: format!("127.0.0.1:{}", app_config.clash_api_port),
            external_ui: String::new(),
            secret: String::new(),
            default_mode: "Rule".into(),
        },
    });

    Ok(SingBoxConfig {
        log: LogConfig {
            disabled: false,
            level: "info".into(),
            timestamp: true,
        },
        dns,
        inbounds,
        outbounds,
        route,
        experimental,
    })
}
