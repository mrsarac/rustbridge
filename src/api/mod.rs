//! REST API module with WebSocket support
//!
//! Provides REST endpoints for reading/writing Modbus registers
//! and WebSocket for real-time register updates.

pub mod auth;

use axum::{
    middleware,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::config::AuthConfig;
use crate::modbus::reader::RegisterStore;

use self::auth::{api_key_auth, AuthState};

/// Broadcast channel capacity for WebSocket updates
const BROADCAST_CAPACITY: usize = 1024;

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub register_store: RegisterStore,
    pub update_tx: broadcast::Sender<RegisterUpdate>,
    pub write_tx: tokio::sync::mpsc::Sender<WriteRequest>,
    pub metrics_handle: Option<PrometheusHandle>,
}

impl ApiState {
    /// Create new API state with broadcast channel
    pub fn new(
        register_store: RegisterStore,
        write_tx: tokio::sync::mpsc::Sender<WriteRequest>,
    ) -> Self {
        let (update_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            register_store,
            update_tx,
            write_tx,
            metrics_handle: None,
        }
    }

    /// Create new API state with metrics handle
    pub fn with_metrics(
        register_store: RegisterStore,
        write_tx: tokio::sync::mpsc::Sender<WriteRequest>,
        metrics_handle: PrometheusHandle,
    ) -> Self {
        let (update_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            register_store,
            update_tx,
            write_tx,
            metrics_handle: Some(metrics_handle),
        }
    }

    /// Get a receiver for register updates
    pub fn subscribe(&self) -> broadcast::Receiver<RegisterUpdate> {
        self.update_tx.subscribe()
    }
}

/// Register update message for WebSocket broadcast
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterUpdate {
    pub device_id: String,
    pub register_name: String,
    pub value: f64,
    pub raw: Vec<u16>,
    pub unit: Option<String>,
    pub timestamp: String,
}

/// Write request sent to Modbus client
#[derive(Debug)]
pub struct WriteRequest {
    pub device_id: String,
    pub address: u16,
    pub value: u16,
    pub response_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
}

/// Create the API router
pub fn create_router(state: ApiState, auth_config: AuthConfig) -> Router {
    let auth_state = Arc::new(AuthState::new(auth_config));

    Router::new()
        // Health & Info
        .route("/health", get(health))
        .route("/api/info", get(api_info))
        // Metrics (Prometheus)
        .route("/metrics", get(metrics_handler))
        // Devices
        .route("/api/devices", get(list_devices))
        .route("/api/devices/:device_id", get(get_device))
        // Registers (read)
        .route("/api/devices/:device_id/registers", get(get_registers))
        .route(
            "/api/devices/:device_id/registers/:register_name",
            get(get_register),
        )
        // Registers (write)
        .route(
            "/api/devices/:device_id/registers/:register_name",
            post(write_register),
        )
        // WebSocket
        .route("/ws", get(ws_handler))
        // Apply API key authentication middleware
        .layer(middleware::from_fn_with_state(
            auth_state,
            api_key_auth,
        ))
        .with_state(Arc::new(state))
}

// ============================================================================
// Error Handling
// ============================================================================

/// API error response
#[derive(Serialize)]
struct ApiError {
    error: String,
    code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl ApiError {
    fn new(code: StatusCode, error: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            code,
            Json(Self {
                error: error.into(),
                code: code.as_u16(),
                details: None,
            }),
        )
    }

    fn with_details(
        code: StatusCode,
        error: impl Into<String>,
        details: impl Into<String>,
    ) -> (StatusCode, Json<Self>) {
        (
            code,
            Json(Self {
                error: error.into(),
                code: code.as_u16(),
                details: Some(details.into()),
            }),
        )
    }
}

// ============================================================================
// Health & Info Endpoints
// ============================================================================

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// API info response
#[derive(Serialize)]
struct ApiInfoResponse {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    endpoints: Vec<EndpointInfo>,
}

#[derive(Serialize)]
struct EndpointInfo {
    method: &'static str,
    path: &'static str,
    description: &'static str,
}

async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        name: "RustBridge API",
        version: env!("CARGO_PKG_VERSION"),
        description: "Industrial Protocol Bridge - Modbus TCP/RTU to JSON/MQTT Gateway",
        endpoints: vec![
            EndpointInfo {
                method: "GET",
                path: "/health",
                description: "Health check",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/info",
                description: "API information",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/devices",
                description: "List all devices",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/devices/:device_id",
                description: "Get device details",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/devices/:device_id/registers",
                description: "List device registers",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/devices/:device_id/registers/:name",
                description: "Get register value",
            },
            EndpointInfo {
                method: "POST",
                path: "/api/devices/:device_id/registers/:name",
                description: "Write register value",
            },
            EndpointInfo {
                method: "GET",
                path: "/ws",
                description: "WebSocket for real-time updates",
            },
            EndpointInfo {
                method: "GET",
                path: "/metrics",
                description: "Prometheus metrics endpoint",
            },
        ],
    })
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match &state.metrics_handle {
        Some(handle) => {
            let metrics = handle.render();
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                metrics,
            )
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            [("content-type", "text/plain; charset=utf-8")],
            "Metrics not enabled".to_string(),
        ),
    }
}

// ============================================================================
// Device Endpoints
// ============================================================================

/// Device list response
#[derive(Serialize)]
struct DeviceListResponse {
    devices: Vec<DeviceSummary>,
    count: usize,
}

#[derive(Serialize)]
struct DeviceSummary {
    id: String,
    register_count: usize,
    last_update: Option<String>,
}

async fn list_devices(State(state): State<Arc<ApiState>>) -> Json<DeviceListResponse> {
    let store = state.register_store.read().await;

    let devices: Vec<DeviceSummary> = store
        .iter()
        .map(|(id, registers)| {
            let last_update = registers
                .values()
                .map(|r| r.timestamp)
                .max()
                .map(|t| t.to_rfc3339());

            DeviceSummary {
                id: id.clone(),
                register_count: registers.len(),
                last_update,
            }
        })
        .collect();

    let count = devices.len();
    Json(DeviceListResponse { devices, count })
}

/// Device detail response
#[derive(Serialize)]
struct DeviceResponse {
    id: String,
    registers: Vec<RegisterResponse>,
    register_count: usize,
}

#[derive(Serialize, Clone)]
struct RegisterResponse {
    name: String,
    value: f64,
    raw: Vec<u16>,
    unit: Option<String>,
    timestamp: String,
}

async fn get_device(
    State(state): State<Arc<ApiState>>,
    Path(device_id): Path<String>,
) -> Result<Json<DeviceResponse>, (StatusCode, Json<ApiError>)> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Device not found"))?;

    let registers: Vec<RegisterResponse> = registers
        .values()
        .map(|r| RegisterResponse {
            name: r.name.clone(),
            value: r.value,
            raw: r.raw.clone(),
            unit: r.unit.clone(),
            timestamp: r.timestamp.to_rfc3339(),
        })
        .collect();

    let register_count = registers.len();
    Ok(Json(DeviceResponse {
        id: device_id,
        registers,
        register_count,
    }))
}

// ============================================================================
// Register Endpoints
// ============================================================================

async fn get_registers(
    State(state): State<Arc<ApiState>>,
    Path(device_id): Path<String>,
) -> Result<Json<Vec<RegisterResponse>>, (StatusCode, Json<ApiError>)> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Device not found"))?;

    let registers: Vec<RegisterResponse> = registers
        .values()
        .map(|r| RegisterResponse {
            name: r.name.clone(),
            value: r.value,
            raw: r.raw.clone(),
            unit: r.unit.clone(),
            timestamp: r.timestamp.to_rfc3339(),
        })
        .collect();

    Ok(Json(registers))
}

async fn get_register(
    State(state): State<Arc<ApiState>>,
    Path((device_id, register_name)): Path<(String, String)>,
) -> Result<Json<RegisterResponse>, (StatusCode, Json<ApiError>)> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Device not found"))?;

    let register = registers
        .get(&register_name)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Register not found"))?;

    Ok(Json(RegisterResponse {
        name: register.name.clone(),
        value: register.value,
        raw: register.raw.clone(),
        unit: register.unit.clone(),
        timestamp: register.timestamp.to_rfc3339(),
    }))
}

/// Write register request body
#[derive(Deserialize)]
struct WriteRegisterRequest {
    /// Raw u16 value to write
    value: u16,
}

/// Write register response
#[derive(Serialize)]
struct WriteRegisterResponse {
    success: bool,
    device_id: String,
    register_name: String,
    value_written: u16,
    message: String,
}

async fn write_register(
    State(state): State<Arc<ApiState>>,
    Path((device_id, register_name)): Path<(String, String)>,
    Json(payload): Json<WriteRegisterRequest>,
) -> Result<Json<WriteRegisterResponse>, (StatusCode, Json<ApiError>)> {
    // Validate device and register exist
    let address = {
        let store = state.register_store.read().await;
        let registers = store
            .get(&device_id)
            .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Device not found"))?;

        let _register = registers
            .get(&register_name)
            .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Register not found"))?;

        // For now, we'll use a placeholder address
        // In production, this would come from the config
        0u16
    };

    // Create response channel
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    // Send write request
    let write_request = WriteRequest {
        device_id: device_id.clone(),
        address,
        value: payload.value,
        response_tx,
    };

    state.write_tx.send(write_request).await.map_err(|_| {
        ApiError::with_details(
            StatusCode::SERVICE_UNAVAILABLE,
            "Write service unavailable",
            "The Modbus write handler is not running",
        )
    })?;

    // Wait for response with timeout
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), response_rx)
        .await
        .map_err(|_| {
            ApiError::with_details(
                StatusCode::GATEWAY_TIMEOUT,
                "Write timeout",
                "The Modbus device did not respond in time",
            )
        })?
        .map_err(|_| {
            ApiError::with_details(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Write failed",
                "Response channel closed unexpectedly",
            )
        })?;

    match result {
        Ok(()) => {
            info!(
                "Write successful: {}:{} = {}",
                device_id, register_name, payload.value
            );
            Ok(Json(WriteRegisterResponse {
                success: true,
                device_id,
                register_name,
                value_written: payload.value,
                message: "Register written successfully".to_string(),
            }))
        }
        Err(e) => Err(ApiError::with_details(
            StatusCode::BAD_GATEWAY,
            "Modbus write failed",
            e,
        )),
    }
}

// ============================================================================
// WebSocket Endpoint
// ============================================================================

/// WebSocket message types
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    /// Subscribe to specific devices/registers
    #[serde(rename = "subscribe")]
    Subscribe { devices: Option<Vec<String>> },
    /// Unsubscribe from updates
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
    /// Register update (server -> client)
    #[serde(rename = "update")]
    Update(RegisterUpdate),
    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
    /// Connection confirmed
    #[serde(rename = "connected")]
    Connected { message: String },
    /// Ping/Pong for keepalive
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<ApiState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<ApiState>) {
    let (mut sender, mut receiver) = socket.split();

    // Send connection confirmation
    let connected_msg = WsMessage::Connected {
        message: format!("RustBridge WebSocket v{}", env!("CARGO_PKG_VERSION")),
    };
    if let Ok(msg) = serde_json::to_string(&connected_msg) {
        if sender.send(Message::Text(msg)).await.is_err() {
            return;
        }
    }

    info!("WebSocket client connected");

    // Subscribe to register updates
    let mut update_rx = state.subscribe();

    // Track subscribed devices (None = all devices)
    let mut subscribed_devices: Option<Vec<String>> = None;

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(WsMessage::Subscribe { devices }) => {
                                subscribed_devices = devices.clone();
                                debug!("Client subscribed to: {:?}", subscribed_devices);
                            }
                            Ok(WsMessage::Unsubscribe) => {
                                subscribed_devices = Some(vec![]);
                                debug!("Client unsubscribed from all updates");
                            }
                            Ok(WsMessage::Ping) => {
                                let pong = serde_json::to_string(&WsMessage::Pong).unwrap();
                                if sender.send(Message::Text(pong)).await.is_err() {
                                    break;
                                }
                            }
                            Ok(_) => {
                                // Ignore other message types from client
                            }
                            Err(e) => {
                                warn!("Invalid WebSocket message: {}", e);
                                let error = WsMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                };
                                if let Ok(msg) = serde_json::to_string(&error) {
                                    let _ = sender.send(Message::Text(msg)).await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            // Handle register updates from broadcast channel
            update = update_rx.recv() => {
                match update {
                    Ok(register_update) => {
                        // Check if client is subscribed to this device
                        let should_send = match &subscribed_devices {
                            None => true, // Subscribed to all
                            Some(devices) if devices.is_empty() => false, // Unsubscribed
                            Some(devices) => devices.contains(&register_update.device_id),
                        };

                        if should_send {
                            let msg = WsMessage::Update(register_update);
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged, missed {} updates", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket connection closed");
}
