//! API Integration Tests for RustBridge
//!
//! Tests the REST API endpoints using tower's ServiceExt

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use rustbridge::api::{create_router, ApiState};
use rustbridge::modbus::reader::{RegisterStore, RegisterValue};

/// Helper to create a test API state
fn create_test_state() -> ApiState {
    let register_store: RegisterStore = Arc::new(RwLock::new(HashMap::new()));
    ApiState { register_store }
}

/// Helper to populate test data
async fn populate_test_data(state: &ApiState) {
    let mut store = state.register_store.write().await;

    // Add device 1 with registers
    let mut device1_registers = HashMap::new();
    device1_registers.insert(
        "temperature".to_string(),
        RegisterValue {
            name: "temperature".to_string(),
            raw: vec![250],
            value: 25.0,
            unit: Some("°C".to_string()),
            timestamp: chrono::Utc::now(),
        },
    );
    device1_registers.insert(
        "humidity".to_string(),
        RegisterValue {
            name: "humidity".to_string(),
            raw: vec![650],
            value: 65.0,
            unit: Some("%".to_string()),
            timestamp: chrono::Utc::now(),
        },
    );
    store.insert("plc-001".to_string(), device1_registers);

    // Add device 2 with registers
    let mut device2_registers = HashMap::new();
    device2_registers.insert(
        "pressure".to_string(),
        RegisterValue {
            name: "pressure".to_string(),
            raw: vec![1000],
            value: 10.0,
            unit: Some("bar".to_string()),
            timestamp: chrono::Utc::now(),
        },
    );
    store.insert("sensor-001".to_string(), device2_registers);
}

/// Helper to make a request and get response body as JSON
async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));

    (status, json)
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_state();
    let app = create_router(state);

    let (status, json) = get_json(app, "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_list_devices_empty() {
    let state = create_test_state();
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["devices"].is_array());
    assert_eq!(json["devices"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_devices_with_data() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);

    let devices = json["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 2);

    // Check that both devices are present
    let device_ids: Vec<&str> = devices
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert!(device_ids.contains(&"plc-001"));
    assert!(device_ids.contains(&"sensor-001"));
}

#[tokio::test]
async fn test_get_device_found() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices/plc-001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"], "plc-001");

    let registers = json["registers"].as_array().unwrap();
    assert_eq!(registers.len(), 2);
}

#[tokio::test]
async fn test_get_device_not_found() {
    let state = create_test_state();
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/devices/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_registers() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices/plc-001/registers").await;

    assert_eq!(status, StatusCode::OK);

    let registers = json.as_array().unwrap();
    assert_eq!(registers.len(), 2);

    // Check register structure
    for reg in registers {
        assert!(reg["name"].is_string());
        assert!(reg["value"].is_number());
        assert!(reg["raw"].is_array());
        assert!(reg["timestamp"].is_string());
    }
}

#[tokio::test]
async fn test_get_single_register() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices/plc-001/registers/temperature").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "temperature");
    assert_eq!(json["value"], 25.0);
    assert_eq!(json["unit"], "°C");
}

#[tokio::test]
async fn test_get_register_not_found() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/devices/plc-001/registers/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_device_register_count() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);

    let devices = json["devices"].as_array().unwrap();

    for device in devices {
        let id = device["id"].as_str().unwrap();
        let count = device["register_count"].as_u64().unwrap();

        match id {
            "plc-001" => assert_eq!(count, 2),
            "sensor-001" => assert_eq!(count, 1),
            _ => panic!("Unexpected device: {}", id),
        }
    }
}

#[tokio::test]
async fn test_device_has_last_update() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);

    let devices = json["devices"].as_array().unwrap();

    for device in devices {
        assert!(device["last_update"].is_string());
        // Verify it's a valid RFC3339 timestamp
        let timestamp = device["last_update"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(timestamp).is_ok());
    }
}

#[tokio::test]
async fn test_register_raw_values() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state);

    let (status, json) = get_json(app, "/api/devices/plc-001/registers/temperature").await;

    assert_eq!(status, StatusCode::OK);

    let raw = json["raw"].as_array().unwrap();
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0], 250);
}

#[tokio::test]
async fn test_health_version_format() {
    let state = create_test_state();
    let app = create_router(state);

    let (status, json) = get_json(app, "/health").await;

    assert_eq!(status, StatusCode::OK);

    // Version should be in semver format (e.g., "0.1.0")
    let version = json["version"].as_str().unwrap();
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(parts.len(), 3, "Version should have 3 parts (major.minor.patch)");

    // Each part should be a number
    for part in parts {
        assert!(part.parse::<u32>().is_ok(), "Version part '{}' should be a number", part);
    }
}
