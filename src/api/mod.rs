//! REST API module

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
// tracing will be used for logging in future versions

use crate::modbus::reader::RegisterStore;

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub register_store: RegisterStore,
}

/// Create the API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/:device_id", get(get_device))
        .route("/api/devices/:device_id/registers", get(get_registers))
        .route("/api/devices/:device_id/registers/:register_name", get(get_register))
        .with_state(Arc::new(state))
}

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

/// Device list response
#[derive(Serialize)]
struct DeviceListResponse {
    devices: Vec<DeviceSummary>,
}

#[derive(Serialize)]
struct DeviceSummary {
    id: String,
    register_count: usize,
    last_update: Option<String>,
}

async fn list_devices(
    State(state): State<Arc<ApiState>>,
) -> Json<DeviceListResponse> {
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

    Json(DeviceListResponse { devices })
}

/// Device detail response
#[derive(Serialize)]
struct DeviceResponse {
    id: String,
    registers: Vec<RegisterResponse>,
}

#[derive(Serialize)]
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
) -> Result<Json<DeviceResponse>, StatusCode> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or(StatusCode::NOT_FOUND)?;

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

    Ok(Json(DeviceResponse {
        id: device_id,
        registers,
    }))
}

async fn get_registers(
    State(state): State<Arc<ApiState>>,
    Path(device_id): Path<String>,
) -> Result<Json<Vec<RegisterResponse>>, StatusCode> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or(StatusCode::NOT_FOUND)?;

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
) -> Result<Json<RegisterResponse>, StatusCode> {
    let store = state.register_store.read().await;

    let registers = store
        .get(&device_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let register = registers
        .get(&register_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(RegisterResponse {
        name: register.name.clone(),
        value: register.value,
        raw: register.raw.clone(),
        unit: register.unit.clone(),
        timestamp: register.timestamp.to_rfc3339(),
    }))
}
