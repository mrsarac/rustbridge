//! API Integration Tests for RustBridge
//!
//! Tests the REST API endpoints using tower's ServiceExt

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use rustbridge::api::{create_router, ApiState};
use rustbridge::config::AuthConfig;
use rustbridge::modbus::reader::{RegisterStore, RegisterValue};

/// Helper to create a disabled auth config for tests
fn disabled_auth() -> AuthConfig {
    AuthConfig {
        enabled: false,
        api_keys: vec![],
        exclude_paths: vec!["/health".to_string(), "/metrics".to_string()],
    }
}

/// Helper to create a test API state
fn create_test_state() -> ApiState {
    let register_store: RegisterStore = Arc::new(RwLock::new(HashMap::new()));
    let (write_tx, _write_rx) = tokio::sync::mpsc::channel(100);
    ApiState::new(register_store, write_tx)
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

/// Helper to make a GET request and get response body as JSON
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

/// Helper to make a POST request with JSON body
async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));

    (status, json)
}

// ============================================================================
// Health Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_health_version_format() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/health").await;

    assert_eq!(status, StatusCode::OK);

    // Version should be in semver format (e.g., "0.1.0")
    let version = json["version"].as_str().unwrap();
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "Version should have 3 parts (major.minor.patch)"
    );

    // Each part should be a number
    for part in parts {
        assert!(
            part.parse::<u32>().is_ok(),
            "Version part '{}' should be a number",
            part
        );
    }
}

// ============================================================================
// API Info Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_api_info_endpoint() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/api/info").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "RustBridge API");
    assert!(json["version"].is_string());
    assert!(json["description"].is_string());
    assert!(json["endpoints"].is_array());

    // Verify endpoints list contains expected entries
    let endpoints = json["endpoints"].as_array().unwrap();
    assert!(endpoints.len() >= 8); // At least 8 endpoints defined
}

// ============================================================================
// Device Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_list_devices_empty() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["devices"].is_array());
    assert_eq!(json["devices"].as_array().unwrap().len(), 0);
    assert_eq!(json["count"], 0);
}

#[tokio::test]
async fn test_list_devices_with_data() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/api/devices").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["count"], 2);

    let devices = json["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 2);

    // Check that both devices are present
    let device_ids: Vec<&str> = devices.iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert!(device_ids.contains(&"plc-001"));
    assert!(device_ids.contains(&"sensor-001"));
}

#[tokio::test]
async fn test_get_device_found() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/api/devices/plc-001").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"], "plc-001");
    assert_eq!(json["register_count"], 2);

    let registers = json["registers"].as_array().unwrap();
    assert_eq!(registers.len(), 2);
}

#[tokio::test]
async fn test_get_device_not_found() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

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

    // Verify error response structure
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "Device not found");
    assert_eq!(json["code"], 404);
}

#[tokio::test]
async fn test_device_register_count() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

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
    let app = create_router(state, disabled_auth());

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

// ============================================================================
// Register Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_registers() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

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
    let app = create_router(state, disabled_auth());

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
    let app = create_router(state, disabled_auth());

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

    // Verify error response structure
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "Register not found");
    assert_eq!(json["code"], 404);
}

#[tokio::test]
async fn test_register_raw_values() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

    let (status, json) = get_json(app, "/api/devices/plc-001/registers/temperature").await;

    assert_eq!(status, StatusCode::OK);

    let raw = json["raw"].as_array().unwrap();
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0], 250);
}

// ============================================================================
// Write Register Tests
// ============================================================================

#[tokio::test]
async fn test_write_register_device_not_found() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    let (status, json) = post_json(
        app,
        "/api/devices/nonexistent/registers/temperature",
        serde_json::json!({"value": 100}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "Device not found");
}

#[tokio::test]
async fn test_write_register_not_found() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

    let (status, json) = post_json(
        app,
        "/api/devices/plc-001/registers/nonexistent",
        serde_json::json!({"value": 100}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "Register not found");
}

// ============================================================================
// WebSocket Tests (Basic)
// ============================================================================

#[tokio::test]
async fn test_websocket_endpoint_exists() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

    // Test that /ws endpoint exists and responds
    // Note: Full WebSocket upgrade requires a real WebSocket client
    // With oneshot(), we just verify the endpoint is routed
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws")
                .header("Upgrade", "websocket")
                .header("Connection", "upgrade")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // With oneshot + hyper, upgrade may return 426 (Upgrade Required)
    // This confirms the endpoint exists and is trying to upgrade
    // A real WebSocket client test would get 101 Switching Protocols
    assert!(
        response.status() == StatusCode::SWITCHING_PROTOCOLS
            || response.status() == StatusCode::UPGRADE_REQUIRED,
        "Expected 101 or 426, got {}",
        response.status()
    );
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn test_error_response_structure() {
    let state = create_test_state();
    let app = create_router(state, disabled_auth());

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

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // All error responses should have these fields
    assert!(json["error"].is_string());
    assert!(json["code"].is_number());
}

// ============================================================================
// API Key Authentication Tests
// ============================================================================

/// Helper to create an enabled auth config for tests
fn enabled_auth_with_keys(keys: Vec<&str>) -> AuthConfig {
    AuthConfig {
        enabled: true,
        api_keys: keys.iter().map(|s| s.to_string()).collect(),
        exclude_paths: vec!["/health".to_string(), "/metrics".to_string()],
    }
}

/// Helper to make a GET request with API key header
async fn get_json_with_key(
    app: axum::Router,
    uri: &str,
    api_key: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().uri(uri);

    if let Some(key) = api_key {
        builder = builder.header("X-API-Key", key);
    }

    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));

    (status, json)
}

#[tokio::test]
async fn test_auth_disabled_allows_all_requests() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, disabled_auth());

    // Should succeed without API key when auth is disabled
    let (status, _) = get_json_with_key(app, "/api/devices", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_auth_enabled_rejects_missing_key() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, enabled_auth_with_keys(vec!["secret-key"]));

    // Should fail without API key
    let (status, json) = get_json_with_key(app, "/api/devices", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "unauthorized");
    assert_eq!(json["message"], "Missing X-API-Key header");
}

#[tokio::test]
async fn test_auth_enabled_rejects_invalid_key() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, enabled_auth_with_keys(vec!["secret-key"]));

    // Should fail with wrong API key
    let (status, json) = get_json_with_key(app, "/api/devices", Some("wrong-key")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "unauthorized");
    assert_eq!(json["message"], "Invalid API key");
}

#[tokio::test]
async fn test_auth_enabled_accepts_valid_key() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, enabled_auth_with_keys(vec!["secret-key"]));

    // Should succeed with valid API key
    let (status, json) = get_json_with_key(app, "/api/devices", Some("secret-key")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["count"], 2);
}

#[tokio::test]
async fn test_auth_multiple_keys() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, enabled_auth_with_keys(vec!["key1", "key2", "key3"]));

    // All keys should work
    let (status, _) = get_json_with_key(app.clone(), "/api/devices", Some("key1")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = get_json_with_key(app.clone(), "/api/devices", Some("key2")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = get_json_with_key(app, "/api/devices", Some("key3")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_auth_excluded_paths_no_key_required() {
    let state = create_test_state();
    let app = create_router(state, enabled_auth_with_keys(vec!["secret-key"]));

    // /health should work without key (excluded path)
    let (status, _) = get_json_with_key(app.clone(), "/health", None).await;
    assert_eq!(status, StatusCode::OK);

    // /metrics should work without key (excluded path)
    let (status, _) = get_json_with_key(app, "/metrics", None).await;
    // Metrics returns 503 if no handle, but not 401
    assert_ne!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_protected_endpoint_requires_key() {
    let state = create_test_state();
    populate_test_data(&state).await;
    let app = create_router(state, enabled_auth_with_keys(vec!["secret-key"]));

    // /api/info should require key (not in excluded paths)
    let (status, _) = get_json_with_key(app.clone(), "/api/info", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // But works with valid key
    let (status, _) = get_json_with_key(app, "/api/info", Some("secret-key")).await;
    assert_eq!(status, StatusCode::OK);
}
