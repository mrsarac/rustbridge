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
#[derive(Debug, Clone, serde::Serialize)]
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
pub fn convert_value(raw: &[u16], config: &RegisterConfig) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegisterType;

    fn make_register_config(data_type: DataType, scale: Option<f64>, offset: Option<f64>) -> RegisterConfig {
        RegisterConfig {
            name: "test".to_string(),
            address: 0,
            register_type: RegisterType::Holding,
            count: 1,
            data_type,
            unit: None,
            scale,
            offset,
        }
    }

    #[test]
    fn test_convert_u16() {
        let config = make_register_config(DataType::U16, None, None);

        assert_eq!(convert_value(&[0], &config), 0.0);
        assert_eq!(convert_value(&[100], &config), 100.0);
        assert_eq!(convert_value(&[65535], &config), 65535.0);
    }

    #[test]
    fn test_convert_i16() {
        let config = make_register_config(DataType::I16, None, None);

        assert_eq!(convert_value(&[0], &config), 0.0);
        assert_eq!(convert_value(&[100], &config), 100.0);
        // 65535 as i16 = -1
        assert_eq!(convert_value(&[65535], &config), -1.0);
        // 65436 as i16 = -100
        assert_eq!(convert_value(&[65436], &config), -100.0);
    }

    #[test]
    fn test_convert_u32() {
        let config = make_register_config(DataType::U32, None, None);

        // 0x00000000
        assert_eq!(convert_value(&[0, 0], &config), 0.0);

        // 0x00010000 = 65536
        assert_eq!(convert_value(&[1, 0], &config), 65536.0);

        // 0x00000001 = 1
        assert_eq!(convert_value(&[0, 1], &config), 1.0);

        // 0x0001FFFF = 131071
        assert_eq!(convert_value(&[1, 65535], &config), 131071.0);

        // 0xFFFFFFFF = 4294967295
        assert_eq!(convert_value(&[65535, 65535], &config), 4294967295.0);
    }

    #[test]
    fn test_convert_i32() {
        let config = make_register_config(DataType::I32, None, None);

        // 0
        assert_eq!(convert_value(&[0, 0], &config), 0.0);

        // 1
        assert_eq!(convert_value(&[0, 1], &config), 1.0);

        // -1 (0xFFFFFFFF)
        assert_eq!(convert_value(&[65535, 65535], &config), -1.0);

        // -100 (0xFFFFFF9C)
        let neg100: i32 = -100;
        let high = ((neg100 as u32) >> 16) as u16;
        let low = (neg100 as u32) as u16;
        assert_eq!(convert_value(&[high, low], &config), -100.0);
    }

    #[test]
    fn test_convert_f32() {
        let config = make_register_config(DataType::F32, None, None);

        // IEEE 754: 0.0
        assert_eq!(convert_value(&[0, 0], &config), 0.0);

        // IEEE 754: 1.0 = 0x3F800000
        let one_bits: u32 = 1.0_f32.to_bits();
        let high = (one_bits >> 16) as u16;
        let low = one_bits as u16;
        assert!((convert_value(&[high, low], &config) - 1.0).abs() < 0.0001);

        // IEEE 754: 3.14159... = 0x40490FDB
        let pi_bits: u32 = std::f32::consts::PI.to_bits();
        let high = (pi_bits >> 16) as u16;
        let low = pi_bits as u16;
        assert!((convert_value(&[high, low], &config) - std::f64::consts::PI).abs() < 0.0001);

        // Negative value: -42.5
        let neg_bits: u32 = (-42.5_f32).to_bits();
        let high = (neg_bits >> 16) as u16;
        let low = neg_bits as u16;
        assert!((convert_value(&[high, low], &config) - (-42.5)).abs() < 0.0001);
    }

    #[test]
    fn test_convert_bool() {
        let config = make_register_config(DataType::Bool, None, None);

        assert_eq!(convert_value(&[0], &config), 0.0);
        assert_eq!(convert_value(&[1], &config), 1.0);
        assert_eq!(convert_value(&[100], &config), 1.0);
        assert_eq!(convert_value(&[65535], &config), 1.0);
    }

    #[test]
    fn test_scale_factor() {
        // Temperature sensor: raw value * 0.1 = actual temperature
        let config = make_register_config(DataType::U16, Some(0.1), None);

        assert_eq!(convert_value(&[250], &config), 25.0);
        assert_eq!(convert_value(&[1000], &config), 100.0);
    }

    #[test]
    fn test_offset() {
        // Sensor with offset calibration
        let config = make_register_config(DataType::I16, None, Some(100.0));

        assert_eq!(convert_value(&[0], &config), 100.0);
        assert_eq!(convert_value(&[50], &config), 150.0);
    }

    #[test]
    fn test_scale_and_offset() {
        // Temperature sensor: (raw * 0.1) + (-40) for Celsius
        let config = make_register_config(DataType::U16, Some(0.1), Some(-40.0));

        // Raw 400 = 40.0 - 40.0 = 0.0°C
        assert_eq!(convert_value(&[400], &config), 0.0);

        // Raw 650 = 65.0 - 40.0 = 25.0°C
        assert_eq!(convert_value(&[650], &config), 25.0);
    }

    #[test]
    fn test_empty_raw_values() {
        let config = make_register_config(DataType::U16, None, None);
        assert_eq!(convert_value(&[], &config), 0.0);

        let config32 = make_register_config(DataType::U32, None, None);
        assert_eq!(convert_value(&[], &config32), 0.0);
        assert_eq!(convert_value(&[1], &config32), 0.0); // Not enough values
    }

    #[test]
    fn test_register_value_creation() {
        let reg_value = RegisterValue {
            name: "temperature".to_string(),
            raw: vec![250],
            value: 25.0,
            unit: Some("°C".to_string()),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(reg_value.name, "temperature");
        assert_eq!(reg_value.value, 25.0);
        assert_eq!(reg_value.unit, Some("°C".to_string()));
    }

    #[test]
    fn test_industrial_temperature_sensor() {
        // Typical industrial temperature sensor:
        // - Returns raw value as 10x actual temperature
        // - Range: -40°C to 125°C
        // - Signed value for negative temperatures
        let config = make_register_config(DataType::I16, Some(0.1), None);

        // -40°C = raw -400
        let raw_neg40: u16 = (-400_i16) as u16;
        assert!((convert_value(&[raw_neg40], &config) - (-40.0)).abs() < 0.01);

        // 0°C = raw 0
        assert_eq!(convert_value(&[0], &config), 0.0);

        // 25°C = raw 250
        assert!((convert_value(&[250], &config) - 25.0).abs() < 0.01);

        // 125°C = raw 1250
        assert!((convert_value(&[1250], &config) - 125.0).abs() < 0.01);
    }

    #[test]
    fn test_pressure_sensor_psi() {
        // Pressure sensor: 0-10000 raw = 0-100 PSI
        let config = make_register_config(DataType::U16, Some(0.01), None);

        assert_eq!(convert_value(&[0], &config), 0.0);
        assert_eq!(convert_value(&[5000], &config), 50.0);
        assert_eq!(convert_value(&[10000], &config), 100.0);
    }

    #[test]
    fn test_flow_meter_with_u32() {
        // Flow meter: 32-bit counter in liters
        let config = make_register_config(DataType::U32, None, None);

        // 1,000,000 liters
        let value: u32 = 1_000_000;
        let high = (value >> 16) as u16;
        let low = value as u16;
        assert_eq!(convert_value(&[high, low], &config), 1_000_000.0);
    }
}
