//! API Key Authentication Middleware
//!
//! Provides tower-compatible middleware for API key validation.
//! Keys are passed via the `X-API-Key` header.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::config::AuthConfig;

/// Authentication state shared across requests
#[derive(Clone)]
pub struct AuthState {
    pub config: AuthConfig,
}

impl AuthState {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Check if the given API key is valid
    pub fn is_valid_key(&self, key: &str) -> bool {
        self.config.api_keys.iter().any(|k| k == key)
    }

    /// Check if the path is excluded from authentication
    pub fn is_excluded_path(&self, path: &str) -> bool {
        self.config.exclude_paths.iter().any(|p| {
            // Support exact match or prefix match for paths ending with *
            if p.ends_with('*') {
                let prefix = &p[..p.len() - 1];
                path.starts_with(prefix)
            } else {
                path == p
            }
        })
    }
}

/// Error response for authentication failures
#[derive(Serialize)]
struct AuthError {
    error: String,
    message: String,
}

/// API Key authentication middleware
///
/// Validates the `X-API-Key` header against configured API keys.
/// Paths in `exclude_paths` are allowed without authentication.
pub async fn api_key_auth(
    State(auth_state): State<Arc<AuthState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Skip auth if disabled
    if !auth_state.config.enabled {
        return next.run(request).await;
    }

    let path = request.uri().path();

    // Skip auth for excluded paths
    if auth_state.is_excluded_path(path) {
        return next.run(request).await;
    }

    // Check for API key header
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) if auth_state.is_valid_key(key) => {
            // Valid key, proceed
            next.run(request).await
        }
        Some(_) => {
            // Invalid key
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Invalid API key".to_string(),
                }),
            )
                .into_response()
        }
        None => {
            // Missing key
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Missing X-API-Key header".to_string(),
                }),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_key() {
        let config = AuthConfig {
            enabled: true,
            api_keys: vec!["secret-key-123".to_string(), "another-key".to_string()],
            exclude_paths: vec!["/health".to_string()],
        };
        let state = AuthState::new(config);

        assert!(state.is_valid_key("secret-key-123"));
        assert!(state.is_valid_key("another-key"));
        assert!(!state.is_valid_key("wrong-key"));
        assert!(!state.is_valid_key(""));
    }

    #[test]
    fn test_excluded_paths_exact() {
        let config = AuthConfig {
            enabled: true,
            api_keys: vec![],
            exclude_paths: vec!["/health".to_string(), "/metrics".to_string()],
        };
        let state = AuthState::new(config);

        assert!(state.is_excluded_path("/health"));
        assert!(state.is_excluded_path("/metrics"));
        assert!(!state.is_excluded_path("/api/devices"));
        assert!(!state.is_excluded_path("/health/detailed"));
    }

    #[test]
    fn test_excluded_paths_wildcard() {
        let config = AuthConfig {
            enabled: true,
            api_keys: vec![],
            exclude_paths: vec!["/public/*".to_string(), "/docs/*".to_string()],
        };
        let state = AuthState::new(config);

        assert!(state.is_excluded_path("/public/info"));
        assert!(state.is_excluded_path("/public/assets/logo.png"));
        assert!(state.is_excluded_path("/docs/api"));
        assert!(!state.is_excluded_path("/api/devices"));
    }

    #[test]
    fn test_empty_keys() {
        let config = AuthConfig {
            enabled: true,
            api_keys: vec![],
            exclude_paths: vec![],
        };
        let state = AuthState::new(config);

        assert!(!state.is_valid_key("any-key"));
    }
}
