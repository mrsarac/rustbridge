//! Modbus client context types

use tokio_modbus::client::Context as TcpContext;

/// Unified context for TCP and RTU clients
pub enum Context {
    Tcp(TcpContext),
    // Rtu will be added in Week 2
}

impl Context {
    pub async fn read_holding_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>, std::io::Error> {
        match self {
            Context::Tcp(ctx) => {
                use tokio_modbus::prelude::*;
                ctx.read_holding_registers(addr, cnt).await
            }
        }
    }

    pub async fn read_input_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>, std::io::Error> {
        match self {
            Context::Tcp(ctx) => {
                use tokio_modbus::prelude::*;
                ctx.read_input_registers(addr, cnt).await
            }
        }
    }

    pub async fn read_coils(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>, std::io::Error> {
        match self {
            Context::Tcp(ctx) => {
                use tokio_modbus::prelude::*;
                ctx.read_coils(addr, cnt).await
            }
        }
    }

    pub async fn read_discrete_inputs(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>, std::io::Error> {
        match self {
            Context::Tcp(ctx) => {
                use tokio_modbus::prelude::*;
                ctx.read_discrete_inputs(addr, cnt).await
            }
        }
    }

    pub async fn write_single_register(&mut self, addr: u16, value: u16) -> Result<(), std::io::Error> {
        match self {
            Context::Tcp(ctx) => {
                use tokio_modbus::prelude::*;
                ctx.write_single_register(addr, value).await
            }
        }
    }
}
