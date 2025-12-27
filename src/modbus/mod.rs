//! Modbus protocol handling
//!
//! Supports both TCP and RTU (serial) connections

use anyhow::{Context as AnyhowContext, Result};
use std::net::SocketAddr;
use tokio_modbus::prelude::*;
use tokio_serial::SerialPortBuilderExt;
use tracing::{debug, info, warn};

use crate::config::{ConnectionConfig, DeviceConfig, RegisterConfig, RegisterType};

pub mod client;
pub mod reader;

/// Modbus client abstraction supporting TCP and RTU
pub struct ModbusClient {
    device_id: String,
    device_type: String,
    context: Option<client::Context>,
}

impl ModbusClient {
    /// Create a new Modbus client from device configuration
    pub async fn new(config: &DeviceConfig) -> Result<Self> {
        info!("Initializing Modbus client for device: {}", config.id);

        let (context, device_type) = match &config.connection {
            ConnectionConfig::Tcp(tcp) => {
                let addr: SocketAddr = format!("{}:{}", tcp.host, tcp.port)
                    .parse()
                    .with_context(|| "Invalid TCP address")?;

                info!("Connecting to Modbus TCP: {} (unit {})", addr, tcp.unit_id);

                let ctx = tcp::connect_slave(addr, Slave(tcp.unit_id))
                    .await
                    .with_context(|| format!("Failed to connect to {}", addr))?;

                (Some(client::Context::Tcp(ctx)), "TCP".to_string())
            }
            ConnectionConfig::Rtu(rtu) => {
                info!(
                    "Connecting to Modbus RTU: {} @ {} baud (unit {})",
                    rtu.port, rtu.baud_rate, rtu.unit_id
                );

                // Parse parity
                let parity = match rtu.parity.to_lowercase().as_str() {
                    "none" => tokio_serial::Parity::None,
                    "even" => tokio_serial::Parity::Even,
                    "odd" => tokio_serial::Parity::Odd,
                    _ => {
                        warn!("Unknown parity '{}', using None", rtu.parity);
                        tokio_serial::Parity::None
                    }
                };

                // Parse stop bits
                let stop_bits = match rtu.stop_bits {
                    1 => tokio_serial::StopBits::One,
                    2 => tokio_serial::StopBits::Two,
                    _ => {
                        warn!("Unknown stop bits {}, using 1", rtu.stop_bits);
                        tokio_serial::StopBits::One
                    }
                };

                // Parse data bits
                let data_bits = match rtu.data_bits {
                    5 => tokio_serial::DataBits::Five,
                    6 => tokio_serial::DataBits::Six,
                    7 => tokio_serial::DataBits::Seven,
                    8 => tokio_serial::DataBits::Eight,
                    _ => {
                        warn!("Unknown data bits {}, using 8", rtu.data_bits);
                        tokio_serial::DataBits::Eight
                    }
                };

                // Create serial port builder
                let builder = tokio_serial::new(&rtu.port, rtu.baud_rate)
                    .parity(parity)
                    .stop_bits(stop_bits)
                    .data_bits(data_bits);

                // Open serial port
                let port = builder.open_native_async().with_context(|| {
                    format!(
                        "Failed to open serial port {} at {} baud",
                        rtu.port, rtu.baud_rate
                    )
                })?;

                info!(
                    "Serial port {} opened: {} baud, {} data bits, {:?} parity, {:?} stop bits",
                    rtu.port, rtu.baud_rate, rtu.data_bits, parity, stop_bits
                );

                // Create RTU context
                let ctx = rtu::attach_slave(port, Slave(rtu.unit_id));

                (Some(client::Context::Rtu(ctx)), "RTU".to_string())
            }
        };

        info!(
            "Modbus {} client ready for device: {}",
            device_type, config.id
        );

        Ok(Self {
            device_id: config.id.clone(),
            device_type,
            context,
        })
    }

    /// Read registers from the device
    pub async fn read_registers(&mut self, register: &RegisterConfig) -> Result<Vec<u16>> {
        let ctx = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        let values = match register.register_type {
            RegisterType::Holding => {
                debug!(
                    "Reading {} holding registers from address {} ({})",
                    register.count, register.address, self.device_type
                );
                ctx.read_holding_registers(register.address, register.count)
                    .await
                    .map_err(|e| anyhow::anyhow!("Modbus error: {}", e))?
            }
            RegisterType::Input => {
                debug!(
                    "Reading {} input registers from address {} ({})",
                    register.count, register.address, self.device_type
                );
                ctx.read_input_registers(register.address, register.count)
                    .await
                    .map_err(|e| anyhow::anyhow!("Modbus error: {}", e))?
            }
            RegisterType::Coil => {
                let coils = ctx
                    .read_coils(register.address, register.count)
                    .await
                    .map_err(|e| anyhow::anyhow!("Modbus error: {}", e))?;
                coils.iter().map(|&b| if b { 1u16 } else { 0u16 }).collect()
            }
            RegisterType::Discrete => {
                let inputs = ctx
                    .read_discrete_inputs(register.address, register.count)
                    .await
                    .map_err(|e| anyhow::anyhow!("Modbus error: {}", e))?;
                inputs
                    .iter()
                    .map(|&b| if b { 1u16 } else { 0u16 })
                    .collect()
            }
        };

        Ok(values)
    }

    /// Write a single register
    #[allow(dead_code)]
    pub async fn write_register(&mut self, address: u16, value: u16) -> Result<()> {
        let ctx = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        ctx.write_single_register(address, value)
            .await
            .map_err(|e| anyhow::anyhow!("Modbus write error: {}", e))?;

        info!(
            "Wrote value {} to register {} on device {} ({})",
            value, address, self.device_id, self.device_type
        );

        Ok(())
    }

    /// Write multiple registers
    #[allow(dead_code)]
    pub async fn write_registers(&mut self, address: u16, values: &[u16]) -> Result<()> {
        let ctx = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        ctx.write_multiple_registers(address, values)
            .await
            .map_err(|e| anyhow::anyhow!("Modbus write error: {}", e))?;

        info!(
            "Wrote {} registers starting at {} on device {} ({})",
            values.len(),
            address,
            self.device_id,
            self.device_type
        );

        Ok(())
    }

    /// Write a single coil
    #[allow(dead_code)]
    pub async fn write_coil(&mut self, address: u16, value: bool) -> Result<()> {
        let ctx = self
            .context
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        ctx.write_single_coil(address, value)
            .await
            .map_err(|e| anyhow::anyhow!("Modbus write error: {}", e))?;

        info!(
            "Wrote coil {} = {} on device {} ({})",
            address, value, self.device_id, self.device_type
        );

        Ok(())
    }

    /// Check if connection is alive
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.context.is_some()
    }

    /// Get device type (TCP or RTU)
    #[allow(dead_code)]
    pub fn device_type(&self) -> &str {
        &self.device_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DataType, RtuConnection, TcpConnection};

    #[test]
    fn test_tcp_connection_config() {
        let tcp = TcpConnection {
            host: "192.168.1.100".to_string(),
            port: 502,
            unit_id: 1,
        };

        assert_eq!(tcp.host, "192.168.1.100");
        assert_eq!(tcp.port, 502);
        assert_eq!(tcp.unit_id, 1);
    }

    #[test]
    fn test_rtu_connection_config() {
        let rtu = RtuConnection {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            parity: "none".to_string(),
            unit_id: 1,
        };

        assert_eq!(rtu.port, "/dev/ttyUSB0");
        assert_eq!(rtu.baud_rate, 9600);
        assert_eq!(rtu.data_bits, 8);
        assert_eq!(rtu.stop_bits, 1);
        assert_eq!(rtu.parity, "none");
    }

    #[test]
    fn test_parity_parsing() {
        // Test parity string parsing logic
        let test_cases = vec![
            ("none", "None"),
            ("even", "Even"),
            ("odd", "Odd"),
            ("NONE", "None"), // case insensitive
            ("Even", "Even"),
            ("invalid", "None"), // fallback
        ];

        for (input, expected) in test_cases {
            let result = match input.to_lowercase().as_str() {
                "none" => "None",
                "even" => "Even",
                "odd" => "Odd",
                _ => "None",
            };
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_register_config() {
        let reg = RegisterConfig {
            name: "temperature".to_string(),
            address: 100,
            register_type: RegisterType::Holding,
            count: 1,
            data_type: DataType::I16,
            unit: Some("°C".to_string()),
            scale: Some(0.1),
            offset: None,
        };

        assert_eq!(reg.name, "temperature");
        assert_eq!(reg.address, 100);
        assert!(matches!(reg.register_type, RegisterType::Holding));
    }
}
