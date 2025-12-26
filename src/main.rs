//! RustBridge - Industrial Protocol Bridge
//!
//! High-performance Modbus TCP/RTU to JSON/MQTT gateway
//! Built with Rust for Industry 4.0 edge deployments

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod modbus;
mod mqtt;
mod api;
mod bridge;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    print_banner();

    info!("Starting RustBridge v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = config::load_config()?;
    info!("Configuration loaded: {} devices configured", config.devices.len());

    // Initialize bridge
    let bridge = bridge::Bridge::new(config).await?;

    // Start the bridge
    bridge.run().await?;

    Ok(())
}

fn print_banner() {
    println!(r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ██████╗ ██╗   ██╗███████╗████████╗██████╗ ██████╗ ██╗██████╗ ║
    ║   ██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██╔══██╗██║██╔═══╝ ║
    ║   ██████╔╝██║   ██║███████╗   ██║   ██████╔╝██████╔╝██║██║  █╗ ║
    ║   ██╔══██╗██║   ██║╚════██║   ██║   ██╔══██╗██╔══██╗██║██║  ██╗║
    ║   ██║  ██║╚██████╔╝███████║   ██║   ██████╔╝██║  ██║██║██████╔╝║
    ║   ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═════╝ ╚═╝  ╚═╝╚═╝╚═════╝ ║
    ║                                                               ║
    ║   Industrial Protocol Bridge                                  ║
    ║   Modbus TCP/RTU → JSON/MQTT Gateway                         ║
    ║   https://github.com/mrsarac/rustbridge                       ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#);
}
