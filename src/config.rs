//! Configuration management for RustBridge

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// MQTT broker configuration
    pub mqtt: MqttConfig,
    /// API authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,
    /// List of Modbus devices
    pub devices: Vec<DeviceConfig>,
}

/// API Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable API key authentication
    #[serde(default)]
    pub enabled: bool,
    /// List of valid API keys
    #[serde(default)]
    pub api_keys: Vec<String>,
    /// Paths excluded from authentication (e.g., /health, /metrics)
    #[serde(default = "AuthConfig::default_exclude_paths")]
    pub exclude_paths: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: vec![],
            exclude_paths: Self::default_exclude_paths(),
        }
    }
}

impl AuthConfig {
    fn default_exclude_paths() -> Vec<String> {
        vec![
            "/health".to_string(),
            "/metrics".to_string(),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP API host
    pub host: String,
    /// HTTP API port
    pub port: u16,
    /// Enable metrics endpoint
    pub metrics_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// Enable MQTT publishing
    #[serde(default)]
    pub enabled: bool,
    /// MQTT broker host
    pub host: String,
    /// MQTT broker port
    pub port: u16,
    /// Client ID
    pub client_id: String,
    /// Topic prefix
    pub topic_prefix: String,
    /// QoS level (0, 1, or 2)
    pub qos: u8,
    /// Retain messages (for status updates)
    #[serde(default)]
    pub retain: bool,
    /// Username (optional)
    pub username: Option<String>,
    /// Password (optional)
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Unique device ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Device type: "tcp" or "rtu"
    pub device_type: DeviceType,
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    /// Registers to read
    pub registers: Vec<RegisterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Tcp,
    Rtu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConnectionConfig {
    Tcp(TcpConnection),
    Rtu(RtuConnection),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnection {
    /// Host address
    pub host: String,
    /// Port (default: 502)
    pub port: u16,
    /// Modbus unit ID
    pub unit_id: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtuConnection {
    /// Serial port path (e.g., /dev/ttyUSB0)
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits
    pub data_bits: u8,
    /// Stop bits
    pub stop_bits: u8,
    /// Parity: "none", "even", "odd"
    pub parity: String,
    /// Modbus unit ID
    pub unit_id: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterConfig {
    /// Register name
    pub name: String,
    /// Register address
    pub address: u16,
    /// Register type: "holding", "input", "coil", "discrete"
    pub register_type: RegisterType,
    /// Number of registers to read
    pub count: u16,
    /// Data type for interpretation
    pub data_type: DataType,
    /// Unit of measurement (optional)
    pub unit: Option<String>,
    /// Scaling factor (optional)
    pub scale: Option<f64>,
    /// Offset (optional)
    pub offset: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterType {
    Holding,
    Input,
    Coil,
    Discrete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    U16,
    I16,
    U32,
    I32,
    F32,
    Bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                metrics_enabled: true,
            },
            mqtt: MqttConfig {
                enabled: false,
                host: "localhost".to_string(),
                port: 1883,
                client_id: "rustbridge".to_string(),
                topic_prefix: "rustbridge".to_string(),
                qos: 1,
                retain: false,
                username: None,
                password: None,
            },
            auth: AuthConfig::default(),
            devices: vec![],
        }
    }
}

/// Load configuration from file or use defaults
pub fn load_config() -> Result<Config> {
    let config_path =
        std::env::var("RUSTBRIDGE_CONFIG").unwrap_or_else(|_| "config.yaml".to_string());

    if Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path))?;

        let config: Config =
            serde_yaml::from_str(&content).with_context(|| "Failed to parse config file")?;

        Ok(config)
    } else {
        tracing::warn!("Config file not found, using defaults");
        Ok(Config::default())
    }
}

/// Load configuration from a YAML string (used in tests)
#[cfg(test)]
pub fn load_config_from_str(yaml: &str) -> Result<Config> {
    serde_yaml::from_str(yaml).with_context(|| "Failed to parse config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert!(config.server.metrics_enabled);
        assert!(!config.mqtt.enabled); // MQTT disabled by default
        assert_eq!(config.mqtt.host, "localhost");
        assert_eq!(config.mqtt.port, 1883);
        assert_eq!(config.mqtt.qos, 1);
        assert!(!config.mqtt.retain);
        assert!(config.devices.is_empty());
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
server:
  host: "127.0.0.1"
  port: 8080
  metrics_enabled: false
mqtt:
  host: "mqtt.example.com"
  port: 1883
  client_id: "test-client"
  topic_prefix: "test"
  qos: 2
devices: []
"#;
        let config = load_config_from_str(yaml).unwrap();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(!config.server.metrics_enabled);
        assert_eq!(config.mqtt.host, "mqtt.example.com");
        assert_eq!(config.mqtt.qos, 2);
    }

    #[test]
    fn test_parse_tcp_device() {
        let yaml = r#"
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
  - id: "plc-001"
    name: "Test PLC"
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
        data_type: i16
        unit: "°C"
        scale: 0.1
"#;
        let config = load_config_from_str(yaml).unwrap();

        assert_eq!(config.devices.len(), 1);
        let device = &config.devices[0];
        assert_eq!(device.id, "plc-001");
        assert_eq!(device.name, "Test PLC");
        assert_eq!(device.poll_interval_ms, 1000);

        match &device.connection {
            ConnectionConfig::Tcp(tcp) => {
                assert_eq!(tcp.host, "192.168.1.100");
                assert_eq!(tcp.port, 502);
                assert_eq!(tcp.unit_id, 1);
            }
            _ => panic!("Expected TCP connection"),
        }

        assert_eq!(device.registers.len(), 1);
        let reg = &device.registers[0];
        assert_eq!(reg.name, "temperature");
        assert_eq!(reg.address, 0);
        assert_eq!(reg.scale, Some(0.1));
        assert_eq!(reg.unit, Some("°C".to_string()));
    }

    #[test]
    fn test_parse_rtu_device() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 3000
  metrics_enabled: true
mqtt:
  host: ""
  port: 1883
  client_id: "rustbridge"
  topic_prefix: "rustbridge"
  qos: 1
devices:
  - id: "sensor-001"
    name: "RTU Sensor"
    device_type: rtu
    connection:
      port: "/dev/ttyUSB0"
      baud_rate: 9600
      data_bits: 8
      stop_bits: 1
      parity: "none"
      unit_id: 1
    poll_interval_ms: 500
    registers:
      - name: "humidity"
        address: 100
        register_type: input
        count: 1
        data_type: u16
        unit: "%"
"#;
        let config = load_config_from_str(yaml).unwrap();

        assert_eq!(config.devices.len(), 1);
        let device = &config.devices[0];

        match &device.connection {
            ConnectionConfig::Rtu(rtu) => {
                assert_eq!(rtu.port, "/dev/ttyUSB0");
                assert_eq!(rtu.baud_rate, 9600);
                assert_eq!(rtu.data_bits, 8);
                assert_eq!(rtu.parity, "none");
            }
            _ => panic!("Expected RTU connection"),
        }
    }

    #[test]
    fn test_all_register_types() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 3000
  metrics_enabled: true
mqtt:
  host: ""
  port: 1883
  client_id: "rustbridge"
  topic_prefix: "rustbridge"
  qos: 1
devices:
  - id: "test"
    name: "Test"
    device_type: tcp
    connection:
      host: "localhost"
      port: 502
      unit_id: 1
    poll_interval_ms: 1000
    registers:
      - name: "holding_reg"
        address: 0
        register_type: holding
        count: 1
        data_type: u16
      - name: "input_reg"
        address: 10
        register_type: input
        count: 1
        data_type: i16
      - name: "coil_reg"
        address: 20
        register_type: coil
        count: 1
        data_type: bool
      - name: "discrete_reg"
        address: 30
        register_type: discrete
        count: 1
        data_type: bool
"#;
        let config = load_config_from_str(yaml).unwrap();

        let regs = &config.devices[0].registers;
        assert_eq!(regs.len(), 4);

        assert!(matches!(regs[0].register_type, RegisterType::Holding));
        assert!(matches!(regs[1].register_type, RegisterType::Input));
        assert!(matches!(regs[2].register_type, RegisterType::Coil));
        assert!(matches!(regs[3].register_type, RegisterType::Discrete));
    }

    #[test]
    fn test_all_data_types() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 3000
  metrics_enabled: true
mqtt:
  host: ""
  port: 1883
  client_id: "rustbridge"
  topic_prefix: "rustbridge"
  qos: 1
devices:
  - id: "test"
    name: "Test"
    device_type: tcp
    connection:
      host: "localhost"
      port: 502
      unit_id: 1
    poll_interval_ms: 1000
    registers:
      - name: "u16_val"
        address: 0
        register_type: holding
        count: 1
        data_type: u16
      - name: "i16_val"
        address: 1
        register_type: holding
        count: 1
        data_type: i16
      - name: "u32_val"
        address: 2
        register_type: holding
        count: 2
        data_type: u32
      - name: "i32_val"
        address: 4
        register_type: holding
        count: 2
        data_type: i32
      - name: "f32_val"
        address: 6
        register_type: holding
        count: 2
        data_type: f32
      - name: "bool_val"
        address: 8
        register_type: holding
        count: 1
        data_type: bool
"#;
        let config = load_config_from_str(yaml).unwrap();

        let regs = &config.devices[0].registers;
        assert_eq!(regs.len(), 6);

        assert!(matches!(regs[0].data_type, DataType::U16));
        assert!(matches!(regs[1].data_type, DataType::I16));
        assert!(matches!(regs[2].data_type, DataType::U32));
        assert!(matches!(regs[3].data_type, DataType::I32));
        assert!(matches!(regs[4].data_type, DataType::F32));
        assert!(matches!(regs[5].data_type, DataType::Bool));
    }

    #[test]
    fn test_invalid_yaml() {
        let yaml = "this is not valid yaml: [";
        let result = load_config_from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_mqtt_with_auth() {
        let yaml = r#"
server:
  host: "0.0.0.0"
  port: 3000
  metrics_enabled: true
mqtt:
  host: "mqtt.secure.com"
  port: 8883
  client_id: "secure-client"
  topic_prefix: "secure"
  qos: 2
  username: "admin"
  password: "secret123"
devices: []
"#;
        let config = load_config_from_str(yaml).unwrap();

        assert_eq!(config.mqtt.username, Some("admin".to_string()));
        assert_eq!(config.mqtt.password, Some("secret123".to_string()));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Should be able to deserialize back
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.server.port, config.server.port);
        assert_eq!(parsed.mqtt.host, config.mqtt.host);
    }
}
