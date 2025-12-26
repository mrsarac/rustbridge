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
    /// List of Modbus devices
    pub devices: Vec<DeviceConfig>,
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
                host: "localhost".to_string(),
                port: 1883,
                client_id: "rustbridge".to_string(),
                topic_prefix: "rustbridge".to_string(),
                qos: 1,
                username: None,
                password: None,
            },
            devices: vec![],
        }
    }
}

/// Load configuration from file or use defaults
pub fn load_config() -> Result<Config> {
    let config_path = std::env::var("RUSTBRIDGE_CONFIG")
        .unwrap_or_else(|_| "config.yaml".to_string());

    if Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path))?;

        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;

        Ok(config)
    } else {
        tracing::warn!("Config file not found, using defaults");
        Ok(Config::default())
    }
}
