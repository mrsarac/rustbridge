# ⚙️ Configuration Reference

RustBridge uses YAML configuration files. This document describes all available options.

## Configuration File Location

RustBridge looks for configuration in this order:
1. `--config` command line argument
2. `RUSTBRIDGE_CONFIG` environment variable
3. `./config.yaml` (current directory)
4. `/etc/rustbridge/config.yaml`

## Complete Configuration Example

```yaml
# =============================================================================
# SERVER CONFIGURATION
# =============================================================================
server:
  host: "0.0.0.0"           # Listen address (0.0.0.0 = all interfaces)
  port: 3000                 # HTTP port
  metrics_enabled: true      # Enable /metrics endpoint
  cors_enabled: true         # Enable CORS headers
  log_level: "info"          # trace, debug, info, warn, error

# =============================================================================
# MQTT CONFIGURATION (Optional)
# =============================================================================
mqtt:
  enabled: true              # Enable MQTT publishing
  host: "localhost"          # MQTT broker host
  port: 1883                 # MQTT broker port (1883=plain, 8883=TLS)
  client_id: "rustbridge"    # MQTT client identifier
  username: ""               # Optional: MQTT username
  password: ""               # Optional: MQTT password
  topic_prefix: "rustbridge" # Topic prefix (rustbridge/device/register)
  qos: 1                     # QoS level (0, 1, 2)
  retain: false              # Retain messages
  use_tls: false             # Use TLS connection
  # TLS options (when use_tls: true)
  ca_cert: "/path/to/ca.crt"
  client_cert: "/path/to/client.crt"
  client_key: "/path/to/client.key"

# =============================================================================
# API AUTHENTICATION (Optional)
# =============================================================================
auth:
  enabled: false             # Enable API key authentication
  api_keys:                  # List of valid API keys
    - "your-secret-key-1"
    - "your-secret-key-2"
  exclude_paths:             # Paths that don't require authentication
    - "/health"
    - "/metrics"
    - "/public/*"            # Wildcard supported

# =============================================================================
# DEVICE CONFIGURATION
# =============================================================================
devices:
  # -------------------------------------------------------------------------
  # Modbus TCP Device Example
  # -------------------------------------------------------------------------
  - id: "plc-main"                    # Unique device ID (used in API/MQTT)
    name: "Main PLC Controller"       # Human-readable name
    device_type: tcp                   # tcp or rtu
    enabled: true                      # Enable/disable device
    connection:
      host: "192.168.1.100"           # Device IP address
      port: 502                        # Modbus TCP port (default: 502)
      unit_id: 1                       # Modbus slave/unit ID
      timeout_ms: 3000                 # Connection timeout
      retries: 3                       # Retry count on failure
      retry_delay_ms: 1000             # Delay between retries
    poll_interval_ms: 1000             # Polling interval in milliseconds
    registers:
      - name: "temperature"
        address: 0
        register_type: holding         # holding, input, coil, discrete
        count: 1                       # Number of registers to read
        data_type: u16                 # See Data Types section
        unit: "°C"
        scale: 0.1                     # Multiply raw value by this
        offset: 0                      # Add this after scaling
        
      - name: "pressure"
        address: 10
        register_type: holding
        count: 2                       # 2 registers = 32-bit value
        data_type: f32_be              # 32-bit float, big-endian
        unit: "bar"

  # -------------------------------------------------------------------------
  # Modbus RTU (Serial) Device Example
  # -------------------------------------------------------------------------
  - id: "sensor-rtu"
    name: "Serial Sensor"
    device_type: rtu
    connection:
      port: "/dev/ttyUSB0"            # Serial port
      baud_rate: 9600                  # 9600, 19200, 38400, 57600, 115200
      data_bits: 8                     # 7 or 8
      stop_bits: 1                     # 1 or 2
      parity: "none"                   # none, even, odd
      unit_id: 1
      timeout_ms: 1000
    poll_interval_ms: 2000
    registers:
      - name: "humidity"
        address: 0
        register_type: input
        count: 1
        data_type: u16
        unit: "%"
```

## Server Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `host` | string | `0.0.0.0` | Listen address |
| `port` | integer | `3000` | HTTP port |
| `metrics_enabled` | boolean | `true` | Enable Prometheus metrics |
| `cors_enabled` | boolean | `true` | Enable CORS headers |
| `log_level` | string | `info` | Log level |

## MQTT Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable MQTT |
| `host` | string | `localhost` | Broker hostname |
| `port` | integer | `1883` | Broker port |
| `client_id` | string | `rustbridge` | Client identifier |
| `username` | string | `""` | Authentication username |
| `password` | string | `""` | Authentication password |
| `topic_prefix` | string | `rustbridge` | Topic prefix |
| `qos` | integer | `1` | Quality of Service (0-2) |
| `retain` | boolean | `false` | Retain messages |
| `use_tls` | boolean | `false` | Use TLS encryption |

## Device Options

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `id` | string | ✅ | Unique device identifier |
| `name` | string | ✅ | Human-readable name |
| `device_type` | string | ✅ | `tcp` or `rtu` |
| `enabled` | boolean | ❌ | Enable device (default: true) |
| `poll_interval_ms` | integer | ✅ | Polling interval |

### TCP Connection Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `host` | string | - | Device IP address |
| `port` | integer | `502` | Modbus TCP port |
| `unit_id` | integer | `1` | Slave/unit ID |
| `timeout_ms` | integer | `3000` | Connection timeout |
| `retries` | integer | `3` | Retry count |
| `retry_delay_ms` | integer | `1000` | Retry delay |

### RTU Connection Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `port` | string | - | Serial port path |
| `baud_rate` | integer | `9600` | Baud rate |
| `data_bits` | integer | `8` | Data bits (7 or 8) |
| `stop_bits` | integer | `1` | Stop bits (1 or 2) |
| `parity` | string | `none` | Parity (none/even/odd) |
| `unit_id` | integer | `1` | Slave/unit ID |

## Register Options

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `name` | string | ✅ | Register name (used in API) |
| `address` | integer | ✅ | Modbus register address |
| `register_type` | string | ✅ | holding/input/coil/discrete |
| `count` | integer | ❌ | Number of registers (default: 1) |
| `data_type` | string | ❌ | Data type (default: u16) |
| `unit` | string | ❌ | Unit of measurement |
| `scale` | float | ❌ | Scale factor (default: 1.0) |
| `offset` | float | ❌ | Offset after scaling (default: 0) |

## Data Types

| Type | Size | Description |
|------|------|-------------|
| `bool` | 1 bit | Boolean (for coils) |
| `u16` | 16 bit | Unsigned 16-bit integer |
| `i16` | 16 bit | Signed 16-bit integer |
| `u32_be` | 32 bit | Unsigned 32-bit, big-endian |
| `u32_le` | 32 bit | Unsigned 32-bit, little-endian |
| `i32_be` | 32 bit | Signed 32-bit, big-endian |
| `i32_le` | 32 bit | Signed 32-bit, little-endian |
| `f32_be` | 32 bit | Float 32-bit, big-endian |
| `f32_le` | 32 bit | Float 32-bit, little-endian |
| `u64_be` | 64 bit | Unsigned 64-bit, big-endian |
| `u64_le` | 64 bit | Unsigned 64-bit, little-endian |
| `f64_be` | 64 bit | Float 64-bit, big-endian |
| `f64_le` | 64 bit | Float 64-bit, little-endian |
| `string` | variable | ASCII string (use count for length) |

### Byte Order (Endianness)

- `_be` = Big-endian (most significant byte first) - **Most common in Modbus**
- `_le` = Little-endian (least significant byte first)

## Environment Variables

Configuration values can be overridden with environment variables:

| Variable | Description |
|----------|-------------|
| `RUSTBRIDGE_CONFIG` | Config file path |
| `RUSTBRIDGE_HOST` | Server host |
| `RUSTBRIDGE_PORT` | Server port |
| `RUSTBRIDGE_LOG_LEVEL` | Log level |
| `MQTT_HOST` | MQTT broker host |
| `MQTT_PORT` | MQTT broker port |
| `MQTT_USERNAME` | MQTT username |
| `MQTT_PASSWORD` | MQTT password |

## Validation

RustBridge validates configuration at startup:

```bash
# Validate config without starting
./rustbridge --config config.yaml --validate
```

Common validation errors:
- Duplicate device IDs
- Invalid register addresses
- Missing required fields
- Invalid data types
