# 🌉 RustBridge

[![CI](https://github.com/neurabytelabs/rustbridge/actions/workflows/ci.yml/badge.svg)](https://github.com/neurabytelabs/rustbridge/actions/workflows/ci.yml)
[![Release](https://github.com/neurabytelabs/rustbridge/actions/workflows/release.yml/badge.svg)](https://github.com/neurabytelabs/rustbridge/actions/workflows/release.yml)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue.svg)](https://ghcr.io/neurabytelabs/rustbridge)

**Industrial Protocol Bridge - Modbus TCP/RTU to JSON/MQTT Gateway**

RustBridge is a high-performance, lightweight gateway that bridges industrial Modbus devices to modern IoT infrastructure. Built with Rust for reliability, speed, and minimal resource usage.

> 🏭 *"Connecting legacy PLCs to the cloud, one register at a time."*

## 🌐 Live Demo

Try the API without installing anything:

```bash
# Health check
curl https://rustbridge.mustafasarac.com/health

# API info
curl https://rustbridge.mustafasarac.com/api/info

# Prometheus metrics
curl https://rustbridge.mustafasarac.com/metrics
```

**Demo URL:** [rustbridge.mustafasarac.com](https://rustbridge.mustafasarac.com)

## ✨ Features

- **🚀 High Performance** — Handles 1000+ registers per second
- **🔌 Dual Protocol** — Modbus TCP and RTU (serial) support
- **📡 MQTT Publisher** — Real-time data streaming to any MQTT broker
- **🌐 REST API** — JSON endpoints for integration
- **📊 WebSocket** — Real-time updates for dashboards
- **📈 Prometheus Metrics** — Production-ready monitoring
- **🐳 Docker Ready** — Single command deployment
- **⚡ Edge Optimized** — Runs on Raspberry Pi, industrial gateways

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting-started.md) | Quick installation and first steps |
| [Configuration](docs/configuration.md) | Complete configuration reference |
| [API Reference](docs/api-reference.md) | REST API and WebSocket documentation |
| [Modbus Guide](docs/modbus-guide.md) | Modbus protocol deep dive |
| [MQTT Integration](docs/mqtt-integration.md) | MQTT broker setup and topics |
| [Prometheus Metrics](docs/prometheus-metrics.md) | Monitoring and alerting |
| [Deployment](docs/deployment.md) | Production deployment strategies |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions |
| [Examples](docs/examples.md) | Real-world use cases |

## 🚀 Quick Start

### Option 1: Docker (Recommended)

```bash
# Simple run
docker run -d \
  -p 3000:3000 \
  -v ./config.yaml:/app/config.yaml \
  ghcr.io/neurabytelabs/rustbridge:latest

# With Docker Compose (includes MQTT broker)
docker compose up -d
```

### Option 2: Download Binary

Download the latest release for your platform from [GitHub Releases](https://github.com/neurabytelabs/rustbridge/releases):

- **Linux x86_64**: `rustbridge-x86_64-unknown-linux-gnu`
- **Linux ARM64**: `rustbridge-aarch64-unknown-linux-gnu`
- **macOS Intel**: `rustbridge-x86_64-apple-darwin`
- **macOS Apple Silicon**: `rustbridge-aarch64-apple-darwin`

```bash
chmod +x rustbridge-*
./rustbridge-x86_64-unknown-linux-gnu
```

### Option 3: From Source

```bash
git clone https://github.com/neurabytelabs/rustbridge.git
cd rustbridge
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
  enabled: true
  host: "localhost"
  port: 1883
  client_id: "rustbridge"
  topic_prefix: "rustbridge"
  qos: 1
  retain: false

# API Authentication (optional)
auth:
  enabled: true
  api_keys:
    - "your-secret-key-1"
    - "your-secret-key-2"
  exclude_paths:
    - "/health"
    - "/metrics"

devices:
  # Modbus TCP device
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

  # Modbus RTU (Serial) device
  - id: "sensor-01"
    name: "RTU Sensor"
    device_type: rtu
    connection:
      port: "/dev/ttyUSB0"
      baud_rate: 9600
      data_bits: 8
      stop_bits: 1
      parity: "none"
      unit_id: 1
    poll_interval_ms: 2000
    registers:
      - name: "humidity"
        address: 0
        register_type: input
        count: 1
        data_type: u16
        unit: "%"
```

> 📖 See [Configuration Reference](docs/configuration.md) for all options.

## 🔌 API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/metrics` | GET | Prometheus metrics |
| `/api/info` | GET | API information |
| `/api/devices` | GET | List all devices |
| `/api/devices/:id` | GET | Get device details |
| `/api/devices/:id/registers` | GET | Get all register values |
| `/api/devices/:id/registers/:name` | GET/POST | Get/Write register |
| `/ws` | WebSocket | Real-time updates |

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

> 📖 See [API Reference](docs/api-reference.md) for full documentation.

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

> 📖 See [MQTT Integration](docs/mqtt-integration.md) for broker setup.

## 📊 Prometheus Metrics

Available at `/metrics` when `metrics_enabled: true`:

| Metric | Type | Description |
|--------|------|-------------|
| `rustbridge_register_reads_total` | Counter | Total register read attempts |
| `rustbridge_read_duration_seconds` | Histogram | Read latency distribution |
| `rustbridge_register_value` | Gauge | Current register values |
| `rustbridge_errors_total` | Counter | Error count by type |
| `rustbridge_device_connected` | Gauge | Device connection status |
| `rustbridge_poll_cycle_seconds` | Histogram | Poll cycle duration |

> 📖 See [Prometheus Metrics](docs/prometheus-metrics.md) for Grafana dashboards and alerting.

## 🚢 Production Deployment

### Docker Compose (Recommended)

```bash
# Production stack
docker compose up -d

# With monitoring (Prometheus + Grafana)
docker compose --profile monitoring up -d

# With Modbus simulator for testing
docker compose --profile dev up -d
```

Access:
- RustBridge API: http://localhost:3000
- Prometheus: http://localhost:9090 (with monitoring profile)
- Grafana: http://localhost:3001 (admin/rustbridge)

### systemd Service (Bare Metal)

```bash
# Install
cd deploy
sudo ./install.sh

# Control
sudo systemctl start rustbridge
sudo systemctl status rustbridge
sudo journalctl -u rustbridge -f
```

### Kubernetes / Helm

```yaml
# Coming soon - Helm chart
helm repo add rustbridge https://neurabytelabs.github.io/rustbridge
helm install rustbridge rustbridge/rustbridge
```

> 📖 See [Deployment Guide](docs/deployment.md) for HA setup, edge devices, and more.

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        RUSTBRIDGE                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐    ┌──────────┐    ┌────────┐    ┌─────────┐  │
│  │ Modbus  │───▶│ Polling  │───▶│Broadcast│──▶│  MQTT   │  │
│  │TCP/RTU  │    │ Engine   │    │ Channel │   │Publisher│  │
│  └─────────┘    └──────────┘    └────────┘    └─────────┘  │
│                                      │                      │
│                                      ▼                      │
│                      ┌──────────────────────────┐          │
│                      │      REST API + WS       │          │
│                      │    (/api, /ws, /metrics) │          │
│                      └──────────────────────────┘          │
│                                      │                      │
│                                      ▼                      │
│                      ┌──────────────────────────┐          │
│                      │   Prometheus + Grafana   │          │
│                      └──────────────────────────┘          │
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

# Format code
cargo fmt
```

## 📁 Project Structure

```
rustbridge/
├── docs/                # 📚 Documentation
│   ├── getting-started.md
│   ├── configuration.md
│   ├── api-reference.md
│   ├── modbus-guide.md
│   ├── mqtt-integration.md
│   ├── prometheus-metrics.md
│   ├── deployment.md
│   ├── troubleshooting.md
│   └── examples.md
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library exports
│   ├── config.rs        # Configuration parsing
│   ├── bridge.rs        # Main orchestration
│   ├── api/             # REST API + WebSocket
│   ├── modbus/          # Modbus TCP/RTU client
│   ├── mqtt/            # MQTT publisher
│   └── metrics/         # Prometheus metrics
├── deploy/
│   ├── mosquitto/       # MQTT broker config
│   ├── prometheus/      # Prometheus config
│   ├── grafana/         # Grafana provisioning
│   ├── systemd/         # systemd service file
│   ├── install.sh       # Installation script
│   └── uninstall.sh     # Uninstallation script
├── config.yaml          # Example configuration
├── Dockerfile           # Multi-stage build
└── docker-compose.yml   # Full stack deployment
```

## 🔒 Security

- **🔑 API Key Authentication** — Secure endpoints with `X-API-Key` header
- Non-root Docker container
- systemd security hardening
- TLS support for MQTT connections
- Rate limiting (coming soon)

### API Authentication

Enable API key authentication in your `config.yaml`:

```yaml
auth:
  enabled: true
  api_keys:
    - "your-secret-api-key"
  exclude_paths:
    - "/health"
    - "/metrics"
```

Then include the key in requests:

```bash
curl -H "X-API-Key: your-secret-api-key" https://rustbridge.example.com/api/devices
```

> 📖 See [API Reference](docs/api-reference.md) for authentication details.

## 📜 License

MIT License - See [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines.

## 📞 Support

- GitHub Issues: [github.com/neurabytelabs/rustbridge/issues](https://github.com/neurabytelabs/rustbridge/issues)
- Email: mrsarac@gmail.com

---

Built with ❤️ by [NeuraByte Labs](https://neurabytelabs.com)

*Part of the Conatus Protocol - Industrial Edge AI Initiative*
