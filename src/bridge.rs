//! Main bridge orchestration

use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::api::{self, ApiState};
use crate::config::Config;
use crate::modbus::reader::{self, RegisterStore};
use crate::mqtt::MqttPublisher;

/// Main bridge that orchestrates all components
pub struct Bridge {
    config: Config,
    register_store: RegisterStore,
}

impl Bridge {
    /// Create a new bridge instance
    pub async fn new(config: Config) -> Result<Self> {
        let register_store: RegisterStore = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            register_store,
        })
    }

    /// Run the bridge
    pub async fn run(self) -> Result<()> {
        // Start MQTT publisher if configured
        let _mqtt_publisher = if !self.config.mqtt.host.is_empty() {
            Some(MqttPublisher::new(&self.config.mqtt).await?)
        } else {
            None
        };

        // Start polling for each device
        for device in &self.config.devices {
            let store = self.register_store.clone();
            let device_config = device.clone();

            tokio::spawn(async move {
                if let Err(e) = reader::start_polling(device_config, store).await {
                    tracing::error!("Polling error: {}", e);
                }
            });
        }

        // Start API server
        let api_state = ApiState {
            register_store: self.register_store.clone(),
        };

        let app = api::create_router(api_state);

        let addr: SocketAddr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse()?;

        info!("Starting API server on http://{}", addr);
        info!("  - Health check: http://{}/health", addr);
        info!("  - Devices:      http://{}/api/devices", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
