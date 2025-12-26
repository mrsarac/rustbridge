//! Modbus client context types

use tokio_modbus::client::Context as TcpContext;
use tokio_modbus::prelude::*;
use tokio_modbus::Exception;

/// Error type for Modbus operations
#[derive(Debug, thiserror::Error)]
pub enum ModbusError {
    #[error("Modbus exception: {0:?}")]
    Exception(Exception),
    #[error("Transport error: {0}")]
    Transport(#[from] tokio_modbus::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Unified context for TCP and RTU clients
pub enum Context {
    Tcp(TcpContext),
    // Rtu will be added in Week 2
}

impl Context {
    pub async fn read_holding_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>, ModbusError> {
        match self {
            Context::Tcp(ctx) => {
                let result = ctx.read_holding_registers(addr, cnt).await?;
                result.map_err(ModbusError::Exception)
            }
        }
    }

    pub async fn read_input_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>, ModbusError> {
        match self {
            Context::Tcp(ctx) => {
                let result = ctx.read_input_registers(addr, cnt).await?;
                result.map_err(ModbusError::Exception)
            }
        }
    }

    pub async fn read_coils(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>, ModbusError> {
        match self {
            Context::Tcp(ctx) => {
                let result = ctx.read_coils(addr, cnt).await?;
                result.map_err(ModbusError::Exception)
            }
        }
    }

    pub async fn read_discrete_inputs(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>, ModbusError> {
        match self {
            Context::Tcp(ctx) => {
                let result = ctx.read_discrete_inputs(addr, cnt).await?;
                result.map_err(ModbusError::Exception)
            }
        }
    }

    pub async fn write_single_register(&mut self, addr: u16, value: u16) -> Result<(), ModbusError> {
        match self {
            Context::Tcp(ctx) => {
                let result = ctx.write_single_register(addr, value).await?;
                result.map_err(ModbusError::Exception)
            }
        }
    }
}
