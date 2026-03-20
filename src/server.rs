use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::any,
    routing::post,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::Serialize;

use crate::{
    config::{Config, DeviceConfig, ServerConfig},
    wol::wake_device,
};

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    bearer_token: Arc<str>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct DeviceEntry {
    name: String,
    host: String,
    port: u16,
    mac: String,
}

#[derive(Debug, Serialize)]
struct DevicesResponse {
    devices: Vec<DeviceEntry>,
}

#[derive(Debug, Serialize)]
struct WakeResponse {
    status: &'static str,
    name: String,
    host: String,
    port: u16,
    mac: String,
}

pub async fn run(config: Config, override_server: ServerConfig) -> Result<()> {
    let merged = merge_server_config(&config.server, &override_server);
    let bind = merged
        .bind
        .ok_or_else(|| anyhow!("missing server bind address; run `wol init` or pass --bind"))?;
    let cert_path = merged
        .cert_path
        .ok_or_else(|| anyhow!("missing TLS certificate path; run `wol init` or pass --cert"))?;
    let key_path = merged
        .key_path
        .ok_or_else(|| anyhow!("missing TLS key path; run `wol init` or pass --key"))?;
    let bearer_token = merged.bearer_token.ok_or_else(|| {
        anyhow!("missing bearer token; run `wol init` or pass --token / WOL_BEARER_TOKEN")
    })?;

    let tls = RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("failed to load TLS certificate or key")?;
    let addr: SocketAddr = bind
        .parse()
        .with_context(|| format!("invalid bind address: {bind}"))?;
    let state = AppState {
        config: Arc::new(config),
        bearer_token: Arc::from(bearer_token),
    };

    let app = Router::new()
        .route("/healthz", post(health))
        .route("/devices", post(list_devices))
        .route("/wake/{name}", post(wake))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(any(not_found))
        .with_state(state);

    axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
        .context("HTTPS server failed")
}

fn merge_server_config(base: &ServerConfig, overrides: &ServerConfig) -> ServerConfig {
    ServerConfig {
        bind: overrides.bind.clone().or_else(|| base.bind.clone()),
        cert_path: overrides
            .cert_path
            .clone()
            .or_else(|| base.cert_path.clone()),
        key_path: overrides.key_path.clone().or_else(|| base.key_path.clone()),
        bearer_token: overrides
            .bearer_token
            .clone()
            .or_else(|| base.bearer_token.clone()),
    }
}

async fn health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    authorize(&headers, state.bearer_token.as_ref())?;

    Ok((StatusCode::OK, axum::Json(HealthResponse { status: "ok" })))
}

async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    authorize(&headers, state.bearer_token.as_ref())?;

    let devices = state
        .config
        .devices
        .iter()
        .map(|(name, device)| DeviceEntry {
            name: name.clone(),
            host: device.host.clone(),
            port: device.port,
            mac: device.mac.clone(),
        })
        .collect();

    Ok((StatusCode::OK, axum::Json(DevicesResponse { devices })))
}

async fn wake(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    authorize(&headers, state.bearer_token.as_ref())?;

    let device = state
        .config
        .devices
        .get(&name)
        .cloned()
        .ok_or_else(|| ApiError::not_found(format!("unknown device: {name}")))?;

    wake_device(&device)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok((
        StatusCode::OK,
        axum::Json(WakeResponse {
            status: "sent",
            name,
            host: device.host,
            port: device.port,
            mac: device.mac,
        }),
    ))
}

async fn method_not_allowed() -> impl IntoResponse {
    ApiError::method_not_allowed("method not allowed")
}

async fn not_found() -> impl IntoResponse {
    ApiError::not_found("route not found")
}

fn authorize(headers: &HeaderMap, expected_token: &str) -> Result<(), ApiError> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("missing Authorization header"))?;
    let auth = auth
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid Authorization header"))?;
    let provided = auth
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("expected Bearer token"))?;

    if provided == expected_token {
        Ok(())
    } else {
        Err(ApiError::unauthorized("invalid bearer token"))
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn method_not_allowed(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = axum::Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

pub fn build_override_server_config(
    bind: Option<String>,
    cert: Option<std::path::PathBuf>,
    key: Option<std::path::PathBuf>,
    token: Option<String>,
) -> ServerConfig {
    ServerConfig {
        bind,
        cert_path: cert,
        key_path: key,
        bearer_token: token,
    }
}

pub fn init_server_config(
    bind: String,
    cert: std::path::PathBuf,
    key: std::path::PathBuf,
    token: String,
) -> ServerConfig {
    ServerConfig {
        bind: Some(bind),
        cert_path: Some(cert),
        key_path: Some(key),
        bearer_token: Some(token),
    }
}

pub fn device_summary(name: &str, device: &DeviceConfig) -> String {
    format!("{name}\t{}\t{}:{}", device.mac, device.host, device.port)
}
