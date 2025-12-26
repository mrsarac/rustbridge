//! Modbus protocol handling
//!
//! Supports both TCP and RTU connections

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio_modbus::prelude::*;
use tracing::{debug, error, info};

use crate::config::{ConnectionConfig, DeviceConfig, RegisterConfig, RegisterType};

pub mod client;
pub mod reader;

/// Modbus client abstraction
pub struct ModbusClient {
    device_id: String,
    context: Option<client::Context>,
}

impl ModbusClient {
    /// Create a new Modbus client from device configuration
    pub async fn new(config: &DeviceConfig) -> Result<Self> {
        info!("Initializing Modbus client for device: {}", config.id);

        let context = match &config.connection {
            ConnectionConfig::Tcp(tcp) => {
                let addr: SocketAddr = format!("{}:{}", tcp.host, tcp.port)
                    .parse()
                    .with_context(|| "Invalid TCP address")?;

                info!("Connecting to Modbus TCP: {} (unit {})", addr, tcp.unit_id);

                let ctx = tcp::connect_slave(addr, Slave(tcp.unit_id))
                    .await
                    .with_context(|| format!("Failed to connect to {}", addr))?;

                Some(client::Context::Tcp(ctx))
            }
            ConnectionConfig::Rtu(_rtu) => {
                // RTU implementation will be added in Week 2
                info!("RTU support coming in Week 2");
                None
            }
        };

        Ok(Self {
            device_id: config.id.clone(),
            context,
        })
    }

    /// Read registers from the device
    pub async fn read_registers(&mut self, register: &RegisterConfig) -> Result<Vec<u16>> {
        let ctx = self.context.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        let values = match register.register_type {
            RegisterType::Holding => {
                debug!("Reading {} holding registers from address {}",
                       register.count, register.address);
                ctx.read_holding_registers(register.address, register.count).await?
            }
            RegisterType::Input => {
                debug!("Reading {} input registers from address {}",
                       register.count, register.address);
                ctx.read_input_registers(register.address, register.count).await?
            }
            RegisterType::Coil => {
                let coils = ctx.read_coils(register.address, register.count).await?;
                coils.iter().map(|&b| if b { 1u16 } else { 0u16 }).collect()
            }
            RegisterType::Discrete => {
                let inputs = ctx.read_discrete_inputs(register.address, register.count).await?;
                inputs.iter().map(|&b| if b { 1u16 } else { 0u16 }).collect()
            }
        };

        Ok(values)
    }

    /// Write a single register
    pub async fn write_register(&mut self, address: u16, value: u16) -> Result<()> {
        let ctx = self.context.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No connection available"))?;

        ctx.write_single_register(address, value).await?;
        info!("Wrote value {} to register {} on device {}",
              value, address, self.device_id);

        Ok(())
    }

    /// Check if connection is alive
    pub fn is_connected(&self) -> bool {
        self.context.is_some()
    }
}
