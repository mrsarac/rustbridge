//! MQTT publisher module

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::config::MqttConfig;
use crate::modbus::reader::RegisterValue;

/// MQTT Publisher for sending register values
pub struct MqttPublisher {
    client: AsyncClient,
    topic_prefix: String,
    qos: QoS,
}

impl MqttPublisher {
    /// Create a new MQTT publisher
    pub async fn new(config: &MqttConfig) -> Result<Self> {
        let mut mqttoptions = MqttOptions::new(
            &config.client_id,
            &config.host,
            config.port,
        );

        mqttoptions.set_keep_alive(Duration::from_secs(30));

        if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            mqttoptions.set_credentials(user, pass);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

        // Spawn event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        info!("Connected to MQTT broker");
                    }
                    Ok(Event::Incoming(Packet::PingResp)) => {
                        debug!("MQTT ping response");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("MQTT error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        let qos = match config.qos {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => {
                warn!("Invalid QoS level {}, using 1", config.qos);
                QoS::AtLeastOnce
            }
        };

        Ok(Self {
            client,
            topic_prefix: config.topic_prefix.clone(),
            qos,
        })
    }

    /// Publish a register value
    pub async fn publish(&self, device_id: &str, value: &RegisterValue) -> Result<()> {
        let topic = format!("{}/{}/{}", self.topic_prefix, device_id, value.name);

        let payload = serde_json::json!({
            "value": value.value,
            "raw": value.raw,
            "unit": value.unit,
            "timestamp": value.timestamp.to_rfc3339(),
        });

        let payload_str = serde_json::to_string(&payload)
            .with_context(|| "Failed to serialize payload")?;

        self.client
            .publish(&topic, self.qos, false, payload_str.as_bytes())
            .await
            .with_context(|| format!("Failed to publish to {}", topic))?;

        debug!("Published to {}: {}", topic, payload_str);

        Ok(())
    }

    /// Publish device status
    pub async fn publish_status(&self, device_id: &str, online: bool) -> Result<()> {
        let topic = format!("{}/{}/status", self.topic_prefix, device_id);
        let payload = if online { "online" } else { "offline" };

        self.client
            .publish(&topic, self.qos, true, payload.as_bytes())
            .await
            .with_context(|| format!("Failed to publish status to {}", topic))?;

        Ok(())
    }
}
