//! Modbus register reader with polling

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

use crate::config::{DataType, DeviceConfig, RegisterConfig};
use super::ModbusClient;

/// Represents a register value with metadata
#[derive(Debug, Clone)]
pub struct RegisterValue {
    pub name: String,
    pub raw: Vec<u16>,
    pub value: f64,
    pub unit: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Shared state for register values
pub type RegisterStore = Arc<RwLock<HashMap<String, HashMap<String, RegisterValue>>>>;

/// Start polling for a device
pub async fn start_polling(
    config: DeviceConfig,
    store: RegisterStore,
) -> Result<()> {
    let mut client = ModbusClient::new(&config).await?;
    let device_id = config.id.clone();
    let poll_interval = Duration::from_millis(config.poll_interval_ms);

    info!("Starting polling for device {} every {}ms",
          device_id, config.poll_interval_ms);

    let mut ticker = interval(poll_interval);

    loop {
        ticker.tick().await;

        for register in &config.registers {
            match client.read_registers(register).await {
                Ok(raw_values) => {
                    let value = convert_value(&raw_values, register);

                    let reg_value = RegisterValue {
                        name: register.name.clone(),
                        raw: raw_values,
                        value,
                        unit: register.unit.clone(),
                        timestamp: chrono::Utc::now(),
                    };

                    // Store the value
                    {
                        let mut store = store.write().await;
                        let device_map = store
                            .entry(device_id.clone())
                            .or_insert_with(HashMap::new);
                        device_map.insert(register.name.clone(), reg_value.clone());
                    }

                    debug!("Device {} register {} = {} {:?}",
                           device_id, register.name, value, register.unit);
                }
                Err(e) => {
                    error!("Failed to read register {} from {}: {}",
                           register.name, device_id, e);
                }
            }
        }
    }
}

/// Convert raw register values to typed value
fn convert_value(raw: &[u16], config: &RegisterConfig) -> f64 {
    let raw_value: f64 = match config.data_type {
        DataType::U16 => raw.first().copied().unwrap_or(0) as f64,
        DataType::I16 => raw.first().copied().unwrap_or(0) as i16 as f64,
        DataType::U32 => {
            if raw.len() >= 2 {
                ((raw[0] as u32) << 16 | raw[1] as u32) as f64
            } else {
                0.0
            }
        }
        DataType::I32 => {
            if raw.len() >= 2 {
                ((raw[0] as u32) << 16 | raw[1] as u32) as i32 as f64
            } else {
                0.0
            }
        }
        DataType::F32 => {
            if raw.len() >= 2 {
                let bits = (raw[0] as u32) << 16 | raw[1] as u32;
                f32::from_bits(bits) as f64
            } else {
                0.0
            }
        }
        DataType::Bool => {
            if raw.first().copied().unwrap_or(0) != 0 { 1.0 } else { 0.0 }
        }
    };

    // Apply scale and offset
    let scale = config.scale.unwrap_or(1.0);
    let offset = config.offset.unwrap_or(0.0);

    raw_value * scale + offset
}
