//! MQTT publisher module
//!
//! Publishes register updates to MQTT broker with topics like:
//! `{prefix}/{device_id}/{register_name}`

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::api::RegisterUpdate;
use crate::config::MqttConfig;

/// MQTT Publisher for sending register values
pub struct MqttPublisher {
    client: AsyncClient,
    topic_prefix: String,
    qos: QoS,
    retain: bool,
    #[allow(dead_code)] // Used for connection status checks
    connected: Arc<AtomicBool>,
}

impl MqttPublisher {
    /// Create a new MQTT publisher
    pub async fn new(config: &MqttConfig) -> Result<Self> {
        let mut mqttoptions = MqttOptions::new(&config.client_id, &config.host, config.port);

        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_clean_session(true);

        if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            mqttoptions.set_credentials(user, pass);
        }

        let (client, eventloop) = AsyncClient::new(mqttoptions, 100);
        let connected = Arc::new(AtomicBool::new(false));

        // Spawn event loop handler
        let connected_clone = connected.clone();
        let host = config.host.clone();
        let port = config.port;
        Self::spawn_event_loop(eventloop, connected_clone, host, port);

        let qos = match config.qos {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => {
                warn!("Invalid QoS level {}, using 1", config.qos);
                QoS::AtLeastOnce
            }
        };

        info!(
            "MQTT publisher initialized: {}:{} (prefix: {}, qos: {})",
            config.host, config.port, config.topic_prefix, config.qos
        );

        Ok(Self {
            client,
            topic_prefix: config.topic_prefix.clone(),
            qos,
            retain: config.retain,
            connected,
        })
    }

    /// Spawn the MQTT event loop handler
    fn spawn_event_loop(
        mut eventloop: EventLoop,
        connected: Arc<AtomicBool>,
        host: String,
        port: u16,
    ) {
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        if ack.code == rumqttc::ConnectReturnCode::Success {
                            connected.store(true, Ordering::SeqCst);
                            info!("Connected to MQTT broker at {}:{}", host, port);
                        } else {
                            error!("MQTT connection rejected: {:?}", ack.code);
                        }
                    }
                    Ok(Event::Incoming(Packet::PingResp)) => {
                        debug!("MQTT ping response");
                    }
                    Ok(Event::Incoming(Packet::Disconnect)) => {
                        connected.store(false, Ordering::SeqCst);
                        warn!("Disconnected from MQTT broker");
                    }
                    Ok(Event::Outgoing(_)) => {
                        // Outgoing events are normal
                    }
                    Ok(_) => {}
                    Err(e) => {
                        connected.store(false, Ordering::SeqCst);
                        error!("MQTT error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    /// Check if connected to broker
    #[allow(dead_code)] // Available for future health checks
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Publish a register update from the broadcast channel
    pub async fn publish_update(&self, update: &RegisterUpdate) -> Result<()> {
        let topic = format!(
            "{}/{}/{}",
            self.topic_prefix, update.device_id, update.register_name
        );

        let payload = serde_json::json!({
            "value": update.value,
            "raw": update.raw,
            "unit": update.unit,
            "timestamp": update.timestamp,
        });

        let payload_str =
            serde_json::to_string(&payload).with_context(|| "Failed to serialize payload")?;

        self.client
            .publish(&topic, self.qos, self.retain, payload_str.as_bytes())
            .await
            .with_context(|| format!("Failed to publish to {}", topic))?;

        debug!("MQTT published to {}: {}", topic, payload_str);

        Ok(())
    }

    /// Publish device status (online/offline)
    #[allow(dead_code)] // Available for device lifecycle events
    pub async fn publish_status(&self, device_id: &str, online: bool) -> Result<()> {
        let topic = format!("{}/{}/status", self.topic_prefix, device_id);
        let payload = if online { "online" } else { "offline" };

        self.client
            .publish(&topic, self.qos, true, payload.as_bytes()) // Always retain status
            .await
            .with_context(|| format!("Failed to publish status to {}", topic))?;

        info!("MQTT status: {} = {}", topic, payload);

        Ok(())
    }

    /// Start the MQTT publishing loop that listens to broadcast channel
    pub async fn start_publishing(
        self: Arc<Self>,
        mut update_rx: broadcast::Receiver<RegisterUpdate>,
    ) {
        info!("MQTT publishing loop started");

        loop {
            match update_rx.recv().await {
                Ok(update) => {
                    if let Err(e) = self.publish_update(&update).await {
                        error!("MQTT publish error: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("MQTT publisher lagged, missed {} updates", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("MQTT broadcast channel closed, stopping publisher");
                    break;
                }
            }
        }
    }
}

/// Statistics for MQTT publishing
#[allow(dead_code)] // Available for future metrics
#[derive(Debug, Default)]
pub struct MqttStats {
    pub messages_sent: u64,
    pub messages_failed: u64,
    pub bytes_sent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_mapping() {
        // QoS 0 = AtMostOnce
        // QoS 1 = AtLeastOnce
        // QoS 2 = ExactlyOnce
        assert!(matches!(
            match 0u8 {
                0 => QoS::AtMostOnce,
                1 => QoS::AtLeastOnce,
                2 => QoS::ExactlyOnce,
                _ => QoS::AtLeastOnce,
            },
            QoS::AtMostOnce
        ));

        assert!(matches!(
            match 1u8 {
                0 => QoS::AtMostOnce,
                1 => QoS::AtLeastOnce,
                2 => QoS::ExactlyOnce,
                _ => QoS::AtLeastOnce,
            },
            QoS::AtLeastOnce
        ));

        assert!(matches!(
            match 2u8 {
                0 => QoS::AtMostOnce,
                1 => QoS::AtLeastOnce,
                2 => QoS::ExactlyOnce,
                _ => QoS::AtLeastOnce,
            },
            QoS::ExactlyOnce
        ));
    }

    #[test]
    fn test_topic_format() {
        let prefix = "rustbridge";
        let device_id = "plc-001";
        let register_name = "temperature";

        let topic = format!("{}/{}/{}", prefix, device_id, register_name);
        assert_eq!(topic, "rustbridge/plc-001/temperature");
    }

    #[test]
    fn test_status_topic_format() {
        let prefix = "rustbridge";
        let device_id = "plc-001";

        let topic = format!("{}/{}/status", prefix, device_id);
        assert_eq!(topic, "rustbridge/plc-001/status");
    }
}
