# 🔌 API Reference

RustBridge provides a RESTful API and WebSocket interface for accessing device data.

## Base URL

```
http://localhost:3000
```

## Authentication

RustBridge supports optional API key authentication. When enabled, all API endpoints (except excluded paths) require a valid API key.

### Configuration

Enable authentication in `config.yaml`:

```yaml
auth:
  enabled: true
  api_keys:
    - "your-secret-key-1"
    - "your-secret-key-2"
  exclude_paths:
    - "/health"
    - "/metrics"
```

### Using API Keys

Include the API key in the `X-API-Key` header:

```bash
# With authentication
curl -H "X-API-Key: your-secret-key-1" http://localhost:3000/api/devices

# Excluded paths work without authentication
curl http://localhost:3000/health
curl http://localhost:3000/metrics
```

### Authentication Errors

**Missing API Key (401 Unauthorized):**
```json
{
  "error": "unauthorized",
  "message": "Missing X-API-Key header"
}
```

**Invalid API Key (401 Unauthorized):**
```json
{
  "error": "unauthorized",
  "message": "Invalid API key"
}
```

### Wildcard Paths

Exclude paths support wildcards:

```yaml
auth:
  exclude_paths:
    - "/health"
    - "/public/*"    # Matches /public/info, /public/docs/api, etc.
```

## Response Format

All responses are JSON with the following structure:

```json
{
  "data": { ... },
  "timestamp": "2025-12-27T10:30:00Z"
}
```

Error responses:

```json
{
  "error": {
    "code": "DEVICE_NOT_FOUND",
    "message": "Device 'plc-99' not found"
  },
  "timestamp": "2025-12-27T10:30:00Z"
}
```

---

## Health & Info

### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600
}
```

### GET /api/info

API information and capabilities.

**Response:**
```json
{
  "name": "RustBridge",
  "version": "1.0.0",
  "features": {
    "mqtt": true,
    "websocket": true,
    "prometheus": true
  },
  "device_count": 5,
  "register_count": 42
}
```

---

## Devices

### GET /api/devices

List all configured devices.

**Response:**
```json
[
  {
    "id": "plc-main",
    "name": "Main PLC Controller",
    "device_type": "tcp",
    "enabled": true,
    "connected": true,
    "last_poll": "2025-12-27T10:30:00Z",
    "poll_interval_ms": 1000,
    "register_count": 8,
    "error_count": 0
  },
  {
    "id": "sensor-01",
    "name": "Temperature Sensor",
    "device_type": "rtu",
    "enabled": true,
    "connected": true,
    "last_poll": "2025-12-27T10:29:58Z",
    "poll_interval_ms": 2000,
    "register_count": 3,
    "error_count": 0
  }
]
```

### GET /api/devices/:id

Get details of a specific device.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Device ID |

**Response:**
```json
{
  "id": "plc-main",
  "name": "Main PLC Controller",
  "device_type": "tcp",
  "connection": {
    "host": "192.168.1.100",
    "port": 502,
    "unit_id": 1
  },
  "enabled": true,
  "connected": true,
  "last_poll": "2025-12-27T10:30:00Z",
  "poll_interval_ms": 1000,
  "statistics": {
    "total_reads": 86400,
    "successful_reads": 86350,
    "failed_reads": 50,
    "avg_response_ms": 12.5
  }
}
```

---

## Registers

### GET /api/devices/:id/registers

Get all register values for a device.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Device ID |

**Response:**
```json
[
  {
    "name": "temperature",
    "value": 23.5,
    "raw": [235],
    "unit": "°C",
    "address": 0,
    "register_type": "holding",
    "timestamp": "2025-12-27T10:30:00Z",
    "quality": "good"
  },
  {
    "name": "pressure",
    "value": 4.2,
    "raw": [16640, 0],
    "unit": "bar",
    "address": 10,
    "register_type": "holding",
    "timestamp": "2025-12-27T10:30:00Z",
    "quality": "good"
  }
]
```

### GET /api/devices/:id/registers/:name

Get a specific register value.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Device ID |
| `name` | string | Register name |

**Response:**
```json
{
  "name": "temperature",
  "value": 23.5,
  "raw": [235],
  "unit": "°C",
  "address": 0,
  "register_type": "holding",
  "data_type": "u16",
  "scale": 0.1,
  "timestamp": "2025-12-27T10:30:00Z",
  "quality": "good"
}
```

### POST /api/devices/:id/registers/:name

Write a value to a register (holding registers and coils only).

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Device ID |
| `name` | string | Register name |

**Request Body:**
```json
{
  "value": 25.0
}
```

**Response:**
```json
{
  "success": true,
  "name": "setpoint",
  "value": 25.0,
  "raw_written": [250],
  "timestamp": "2025-12-27T10:30:00Z"
}
```

**Error Response (read-only register):**
```json
{
  "error": {
    "code": "REGISTER_READ_ONLY",
    "message": "Register 'temperature' is read-only (input register)"
  }
}
```

---

## WebSocket

### WS /ws

Real-time data stream via WebSocket.

**Connection:**
```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(data);
};
```

**Message Types:**

1. **Register Update**
```json
{
  "type": "register_update",
  "device_id": "plc-main",
  "register": {
    "name": "temperature",
    "value": 23.5,
    "unit": "°C",
    "timestamp": "2025-12-27T10:30:00Z"
  }
}
```

2. **Device Status Change**
```json
{
  "type": "device_status",
  "device_id": "plc-main",
  "connected": true,
  "timestamp": "2025-12-27T10:30:00Z"
}
```

3. **Error Event**
```json
{
  "type": "error",
  "device_id": "plc-main",
  "error": "Connection timeout",
  "timestamp": "2025-12-27T10:30:00Z"
}
```

**Subscription (filter messages):**
```javascript
// Subscribe to specific devices
ws.send(JSON.stringify({
  "action": "subscribe",
  "devices": ["plc-main", "sensor-01"]
}));

// Subscribe to specific registers
ws.send(JSON.stringify({
  "action": "subscribe",
  "devices": ["plc-main"],
  "registers": ["temperature", "pressure"]
}));

// Unsubscribe
ws.send(JSON.stringify({
  "action": "unsubscribe"
}));
```

---

## Metrics

### GET /metrics

Prometheus metrics endpoint.

**Response (text/plain):**
```
# HELP rustbridge_register_value Current register value
# TYPE rustbridge_register_value gauge
rustbridge_register_value{device="plc-main",register="temperature"} 23.5
rustbridge_register_value{device="plc-main",register="pressure"} 4.2

# HELP rustbridge_register_reads_total Total register reads
# TYPE rustbridge_register_reads_total counter
rustbridge_register_reads_total{device="plc-main",status="success"} 86350
rustbridge_register_reads_total{device="plc-main",status="error"} 50

# HELP rustbridge_device_connected Device connection status
# TYPE rustbridge_device_connected gauge
rustbridge_device_connected{device="plc-main"} 1
rustbridge_device_connected{device="sensor-01"} 1
```

See [Prometheus Metrics](prometheus-metrics.md) for the full metrics reference.

---

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `DEVICE_NOT_FOUND` | 404 | Device ID does not exist |
| `REGISTER_NOT_FOUND` | 404 | Register name does not exist |
| `REGISTER_READ_ONLY` | 400 | Cannot write to input register |
| `INVALID_VALUE` | 400 | Value out of range or wrong type |
| `DEVICE_DISCONNECTED` | 503 | Device is not connected |
| `MODBUS_ERROR` | 502 | Modbus communication error |
| `TIMEOUT` | 504 | Request timeout |

---

## Rate Limiting

Currently no rate limiting is applied. Rate limiting is planned for v1.1.0.

## CORS

CORS is enabled by default. All origins are allowed. Configure with `server.cors_enabled: false` to disable.
