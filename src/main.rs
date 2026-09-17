mod compat;
mod models;

use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::net::TcpListener;
use crate::models::{DecodeRequest, DecodeResponse, EncodeRequest, EncodeResponse, ErrorResponse, HealthResponse, ValidateRequest, ValidateResponse};

#[derive(Clone, Default)] struct AppState;
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;
fn error(status: StatusCode, code: &str, message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) { (status, Json(ErrorResponse { code: code.into(), message: message.into() })) }
fn ensure_format(format: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> { if matches!(format, "rbxm" | "rbxl") { Ok(()) } else { Err(error(StatusCode::BAD_REQUEST, "invalid_format", "format must be rbxm or rbxl")) } }

async fn health() -> Json<HealthResponse> { Json(HealthResponse { ok: true, service: "rbxm-api".into(), version: env!("CARGO_PKG_VERSION").into() }) }

async fn encode_rbxm(State(_): State<AppState>, Json(request): Json<EncodeRequest>) -> ApiResult<EncodeResponse> {
    ensure_format(&request.format)?;
    let bytes = compat::encode(&request).map_err(|e| error(StatusCode::BAD_REQUEST, "encode_error", e.to_string()))?;
    Ok(Json(EncodeResponse { format: request.format, encoding: "base64".into(), data: STANDARD.encode(bytes), note: "Encoded by rbx_binary/rbx_dom_weak public compatibility engine".into() }))
}

async fn decode_rbxm(State(_): State<AppState>, Json(request): Json<DecodeRequest>) -> ApiResult<DecodeResponse> {
    ensure_format(&request.format)?;
    if request.encoding != "base64" { return Err(error(StatusCode::BAD_REQUEST, "invalid_encoding", "encoding must be base64")); }
    let bytes = STANDARD.decode(request.data).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_base64", e.to_string()))?;
    let data = compat::decode(&bytes).map_err(|e| error(StatusCode::BAD_REQUEST, "decode_error", e.to_string()))?;
    Ok(Json(DecodeResponse { format: request.format, data, note: "Decoded by rbx_binary/rbx_dom_weak public compatibility engine".into() }))
}

async fn validate_rbxm(State(_): State<AppState>, Json(request): Json<ValidateRequest>) -> ApiResult<ValidateResponse> {
    ensure_format(&request.format)?;
    let bytes = STANDARD.decode(request.data).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_base64", e.to_string()))?;
    match compat::validate(&bytes) { Ok(()) => Ok(Json(ValidateResponse { ok: true, format: request.format, issues: vec![], note: "Valid RBXM/RBXL binary document".into() })), Err(e) => Ok(Json(ValidateResponse { ok: false, format: request.format, issues: vec![e.to_string()], note: "Binary parser rejected the document".into() })) }
}

#[tokio::main]
async fn main() { tracing_subscriber::fmt::init(); let app = Router::new().route("/health", get(health)).route("/v1/rbxm/encode", post(encode_rbxm)).route("/v1/rbxm/decode", post(decode_rbxm)).route("/v1/rbxm/validate", post(validate_rbxm)).with_state(AppState); let listener = TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind port 3000"); tracing::info!("RBXM API listening on http://0.0.0.0:3000"); axum::serve(listener, app).await.expect("server failed"); }
