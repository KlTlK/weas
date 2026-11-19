# weas

Next-generation VPN client written in Rust, powered by **sing-box**.

## Features

- ⚡ **Blazing Fast**: Lightweight, low memory footprint core written in Rust.
- 🛡️ **sing-box Under the Hood**: Robust, production-grade universal proxy platform supporting VLESS, Shadowsocks, Trojan, VMess, WireGuard, Hysteria2, and TUIC.
- 🌐 **TUN & System Proxy**: Full system-wide VPN mode via TUN virtual network adapter or lightweight system proxy.
- 📡 **Subscription Management**: Supports standard Base64 subscriptions, Clash configs, and raw share links.
- 📊 **Real-time Metrics**: Live bandwidth monitoring, connection tracking, and node latency benchmarks via sing-box API.
- 🔀 **Smart Routing**: Built-in GeoIP/GeoSite rule sets to bypass local/LAN traffic and route traffic seamlessly.
- 💻 **Cross-Platform**: Designed for Windows, macOS, and Linux.

## Architecture

```
                  ┌───────────────────────┐
                  │      weas (Rust)      │
                  │  CLI / Daemon / Logic │
                  └───────────┬───────────┘
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│  Config & Node   │ │ Process Manager  │ │  Clash/REST API  │
│ Subscription Lib │ │ & Binary Sync    │ │  Traffic/Latency │
└──────────────────┘ └────────┬─────────┘ └────────┬─────────┘
                              │                    │
                              ▼                    ▼
                    ┌────────────────────────────────┐
                    │       sing-box (Core)          │
                    │   TUN / Inbounds / Outbounds   │
                    └────────────────────────────────┘
```

## Getting Started

### Prerequisites

- [Rust & Cargo](https://rustup.rs/) (1.75+)
- [sing-box](https://github.com/SagerNet/sing-box) (can be auto-installed via `weas core install`)

### Build

```bash
cargo build --release
```

### Quick Usage

```bash
# Add a subscription
weas sub add "https://example.com/api/v1/client/subscribe?token=..."

# List available nodes with ping
weas node list --ping

# Connect in TUN mode (requires Admin / root)
weas run --mode tun

# Or connect in System Proxy mode
weas run --mode sysproxy
```

## License

MIT OR Apache-2.0
