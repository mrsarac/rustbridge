# 🌉 RustBridge

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://hub.docker.com/r/mrsarac/rustbridge)

**Industrial Protocol Bridge - Modbus TCP/RTU to JSON/MQTT Gateway**

RustBridge is a high-performance, lightweight gateway that bridges industrial Modbus devices to modern IoT infrastructure. Built with Rust for reliability, speed, and minimal resource usage.

> 🏭 *"Connecting legacy PLCs to the cloud, one register at a time."*

## ✨ Features

- **🚀 High Performance** — Handles 1000+ registers per second
- **🔌 Dual Protocol** — Modbus TCP and RTU (serial) support
- **📡 MQTT Publisher** — Real-time data streaming to any MQTT broker
- **🌐 REST API** — JSON endpoints for integration
- **📊 Dashboard** — Built-in web UI for monitoring
- **🐳 Docker Ready** — Single command deployment
- **⚡ Edge Optimized** — Runs on Raspberry Pi, industrial gateways

## 🚀 Quick Start

### Option 1: Docker (Recommended)

```bash
docker run -d \
  -p 3000:3000 \
  -v ./config.yaml:/app/config.yaml \
  mrsarac/rustbridge:latest
```

### Option 2: From Source

```bash
# Clone the repository
git clone https://github.com/mrsarac/rustbridge.git
cd rustbridge

# Build and run
cargo build --release
./target/release/rustbridge
```

## 📝 Configuration

Create a `config.yaml` file:

```yaml
server:
  host: "0.0.0.0"
  port: 3000
  metrics_enabled: true

mqtt:
  host: "localhost"
  port: 1883
  client_id: "rustbridge"
  topic_prefix: "rustbridge"
  qos: 1

devices:
  - id: "plc-01"
    name: "Main PLC"
    device_type: tcp
    connection:
      host: "192.168.1.100"
      port: 502
      unit_id: 1
    poll_interval_ms: 1000
    registers:
      - name: "temperature"
        address: 0
        register_type: holding
        count: 1
        data_type: u16
        unit: "°C"
        scale: 0.1
      - name: "pressure"
        address: 1
        register_type: holding
        count: 1
        data_type: u16
        unit: "bar"
        scale: 0.01
```

## 🔌 API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/devices` | GET | List all devices |
| `/api/devices/:id` | GET | Get device details |
| `/api/devices/:id/registers` | GET | Get all register values |
| `/api/devices/:id/registers/:name` | GET | Get specific register |

### Example Response

```json
{
  "name": "temperature",
  "value": 72.4,
  "raw": [724],
  "unit": "°C",
  "timestamp": "2025-12-26T23:15:00Z"
}
```

## 📡 MQTT Topics

Data is published to: `{prefix}/{device_id}/{register_name}`

Example: `rustbridge/plc-01/temperature`

```json
{
  "value": 72.4,
  "raw": [724],
  "unit": "°C",
  "timestamp": "2025-12-26T23:15:00Z"
}
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        RUSTBRIDGE                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐    ┌──────────┐    ┌────────┐    ┌─────────┐  │
│  │ Modbus  │───▶│ Polling  │───▶│ Buffer │───▶│  MQTT   │  │
│  │ Client  │    │ Engine   │    │        │    │ Publisher│  │
│  └─────────┘    └──────────┘    └────────┘    └─────────┘  │
│       │                              │                      │
│       │                              ▼                      │
│       │                        ┌──────────┐                │
│       └───────────────────────▶│ REST API │                │
│                                └──────────┘                │
│                                     │                       │
│                                     ▼                       │
│                               ┌──────────┐                 │
│                               │Dashboard │                 │
│                               └──────────┘                 │
└─────────────────────────────────────────────────────────────┘
```

## 🛠️ Development

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Build release
cargo build --release

# Run clippy
cargo clippy
```

## 📊 Metrics

Prometheus metrics are exposed at `/metrics` when `metrics_enabled: true`.

## 🔒 Security

- TLS support for MQTT connections
- API authentication (coming soon)
- Rate limiting (coming soon)

## 📜 License

MIT License - See [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines.

## 📞 Support

- GitHub Issues: [github.com/mrsarac/rustbridge/issues](https://github.com/mrsarac/rustbridge/issues)
- Email: mrsarac@gmail.com

---

Built with ❤️ by [NeuraByte Labs](https://neurabytelabs.com)

*Part of the Conatus Protocol - Industrial Edge AI Initiative*
