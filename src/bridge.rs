//! Main bridge orchestration

use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::info;

use crate::api::{self, ApiState, RegisterUpdate, WriteRequest};
use crate::config::Config;
use crate::metrics::{self, ReadMetrics};
use crate::modbus::reader::{self, RegisterStore, RegisterValue};
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
        // Create write request channel
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<WriteRequest>(100);

        // Initialize Prometheus metrics if enabled
        let api_state = if self.config.server.metrics_enabled {
            let metrics_handle = metrics::init_metrics();
            info!("Prometheus metrics enabled at /metrics");
            ApiState::with_metrics(self.register_store.clone(), write_tx, metrics_handle)
        } else {
            ApiState::new(self.register_store.clone(), write_tx)
        };

        // Clone for the polling tasks to broadcast updates
        let update_broadcaster = api_state.update_tx.clone();

        // Start MQTT publisher if enabled
        if self.config.mqtt.enabled {
            let mqtt_publisher = Arc::new(MqttPublisher::new(&self.config.mqtt).await?);
            let mqtt_rx = api_state.subscribe();

            // Spawn MQTT publishing loop
            tokio::spawn(async move {
                mqtt_publisher.start_publishing(mqtt_rx).await;
            });

            info!(
                "MQTT publishing enabled: {}:{}/{}",
                self.config.mqtt.host, self.config.mqtt.port, self.config.mqtt.topic_prefix
            );
        } else {
            info!("MQTT publishing disabled");
        }

        // Start polling for each device with WebSocket broadcast
        for device in &self.config.devices {
            let store = self.register_store.clone();
            let device_config = device.clone();
            let broadcaster = update_broadcaster.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    start_polling_with_broadcast(device_config, store, broadcaster).await
                {
                    tracing::error!("Polling error: {}", e);
                }
            });
        }

        // Spawn write request handler
        tokio::spawn(async move {
            while let Some(request) = write_rx.recv().await {
                // For now, acknowledge the write request
                // In production, this would forward to the actual Modbus client
                let _ = request.response_tx.send(Ok(()));
                info!(
                    "Write request received: {}@{} = {}",
                    request.device_id, request.address, request.value
                );
            }
        });

        // Start API server
        let app = api::create_router(api_state, self.config.auth.clone());

        // Log authentication status
        if self.config.auth.enabled {
            info!(
                "API authentication enabled with {} API key(s)",
                self.config.auth.api_keys.len()
            );
        } else {
            info!("API authentication disabled (open access)");
        }

        let addr: SocketAddr =
            format!("{}:{}", self.config.server.host, self.config.server.port).parse()?;

        info!("Starting API server on http://{}", addr);
        info!("  - Health check: http://{}/health", addr);
        info!("  - API info:     http://{}/api/info", addr);
        info!("  - Devices:      http://{}/api/devices", addr);
        info!("  - WebSocket:    ws://{}/ws", addr);
        if self.config.server.metrics_enabled {
            info!("  - Metrics:      http://{}/metrics", addr);
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Start polling with WebSocket broadcast support and metrics
async fn start_polling_with_broadcast(
    config: crate::config::DeviceConfig,
    store: RegisterStore,
    broadcaster: tokio::sync::broadcast::Sender<RegisterUpdate>,
) -> Result<()> {
    use crate::modbus::ModbusClient;
    use tokio::time::{interval, Duration};

    let mut client = ModbusClient::new(&config).await?;
    let device_id = config.id.clone();
    let poll_interval = Duration::from_millis(config.poll_interval_ms);

    info!(
        "Starting polling for device {} every {}ms",
        device_id, config.poll_interval_ms
    );

    // Record device as connected
    metrics::record_device_status(&device_id, true);

    let mut ticker = interval(poll_interval);

    loop {
        ticker.tick().await;
        let cycle_start = Instant::now();

        for register in &config.registers {
            // Start metrics timing
            let read_metrics = ReadMetrics::start(&device_id, &register.name);

            match client.read_registers(register).await {
                Ok(raw_values) => {
                    let value = reader::convert_value(&raw_values, register);

                    // Record successful read metrics
                    read_metrics.success(value);

                    let reg_value = RegisterValue {
                        name: register.name.clone(),
                        raw: raw_values.clone(),
                        value,
                        unit: register.unit.clone(),
                        timestamp: chrono::Utc::now(),
                    };

                    // Store the value
                    {
                        let mut store = store.write().await;
                        let device_map =
                            store.entry(device_id.clone()).or_insert_with(HashMap::new);
                        device_map.insert(register.name.clone(), reg_value.clone());
                    }

                    // Broadcast to WebSocket clients (and MQTT if enabled)
                    let update = RegisterUpdate {
                        device_id: device_id.clone(),
                        register_name: register.name.clone(),
                        value: reg_value.value,
                        raw: reg_value.raw,
                        unit: reg_value.unit,
                        timestamp: reg_value.timestamp.to_rfc3339(),
                    };
                    let _ = broadcaster.send(update);

                    tracing::debug!(
                        "Device {} register {} = {} {:?}",
                        device_id,
                        register.name,
                        value,
                        register.unit
                    );
                }
                Err(e) => {
                    // Record failed read metrics
                    read_metrics.failure("modbus_error");

                    tracing::error!(
                        "Failed to read register {} from {}: {}",
                        register.name,
                        device_id,
                        e
                    );
                }
            }
        }

        // Record poll cycle duration
        let cycle_duration = cycle_start.elapsed().as_millis() as u64;
        metrics::record_poll_cycle(&device_id, cycle_duration);
    }
}
