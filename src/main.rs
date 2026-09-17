mod compat;
mod models;

use axum::{
    body::Bytes,
    extract::State,
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::net::TcpListener;
use crate::models::{DecodeRequest, DecodeResponse, EncodeRequest, EncodeResponse, ErrorResponse, HealthResponse, ValidateRequest, ValidateResponse};

#[derive(Clone, Default)] struct AppState;
type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;
fn error(status: StatusCode, code: &str, message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) { (status, Json(ErrorResponse { code: code.into(), message: message.into() })) }
fn format_ok(format: &str) -> bool { matches!(format, "rbxm" | "rbxl") }
fn binary_response(bytes: Vec<u8>) -> Response { Response::builder().status(StatusCode::OK).header(header::CONTENT_TYPE, "application/octet-stream").header(header::CONTENT_DISPOSITION, "attachment; filename=output.rbxm").body(bytes.into()).unwrap() }

async fn health() -> Json<HealthResponse> { Json(HealthResponse { ok: true, service: "rbxm-api".into(), version: env!("CARGO_PKG_VERSION").into() }) }

async fn encode_json(Json(request): Json<EncodeRequest>) -> ApiResult<EncodeResponse> {
    if !format_ok(&request.format) { return Err(error(StatusCode::BAD_REQUEST, "invalid_format", "format must be rbxm or rbxl")); }
    let bytes = compat::encode(&request).map_err(|e| error(StatusCode::BAD_REQUEST, "encode_error", e.to_string()))?;
    Ok(Json(EncodeResponse { format: request.format, encoding: "base64".into(), data: STANDARD.encode(bytes), note: "Debug JSON adapter; use /v1/rbxm/bytes for raw binary.".into() }))
}

async fn encode_bytes(body: Bytes) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let bytes = compat::bytes_body(body).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_bytes", e.to_string()))?;
    compat::validate(&bytes).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_rbxm", e.to_string()))?;
    Ok(binary_response(bytes))
}

async fn decode_bytes(body: Bytes) -> ApiResult<DecodeResponse> {
    let bytes = compat::bytes_body(body).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_bytes", e.to_string()))?;
    let data = compat::decode(&bytes).map_err(|e| error(StatusCode::BAD_REQUEST, "decode_error", e.to_string()))?;
    Ok(Json(DecodeResponse { format: "rbxm".into(), data, note: "Decoded raw RBXM/RBXL bytes with rbx_binary.".into() }))
}

async fn decode_base64(Json(request): Json<DecodeRequest>) -> ApiResult<DecodeResponse> {
    if request.encoding != "base64" { return Err(error(StatusCode::BAD_REQUEST, "invalid_encoding", "encoding must be base64")); }
    let bytes = STANDARD.decode(request.data).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_base64", e.to_string()))?;
    let data = compat::decode(&bytes).map_err(|e| error(StatusCode::BAD_REQUEST, "decode_error", e.to_string()))?;
    Ok(Json(DecodeResponse { format: request.format, data, note: "Decoded base64 RBXM/RBXL bytes with rbx_binary.".into() }))
}

async fn validate_bytes(body: Bytes) -> ApiResult<ValidateResponse> {
    let bytes = compat::bytes_body(body).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_bytes", e.to_string()))?;
    match compat::validate(&bytes) { Ok(()) => Ok(Json(ValidateResponse { ok: true, format: "rbxm".into(), issues: vec![], note: "Valid RBXM/RBXL binary document.".into() })), Err(e) => Ok(Json(ValidateResponse { ok: false, format: "rbxm".into(), issues: vec![e.to_string()], note: "rbx_binary rejected the document.".into() })) }
}

async fn validate_base64(Json(request): Json<ValidateRequest>) -> ApiResult<ValidateResponse> {
    if request.encoding != "base64" { return Err(error(StatusCode::BAD_REQUEST, "invalid_encoding", "encoding must be base64")); }
    let bytes = STANDARD.decode(request.data).map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_base64", e.to_string()))?;
    match compat::validate(&bytes) { Ok(()) => Ok(Json(ValidateResponse { ok: true, format: request.format, issues: vec![], note: "Valid RBXM/RBXL binary document.".into() })), Err(e) => Ok(Json(ValidateResponse { ok: false, format: request.format, issues: vec![e.to_string()], note: "rbx_binary rejected the document.".into() })) }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/rbxm/encode", post(encode_json))
        .route("/v1/rbxm/bytes", post(encode_bytes))
        .route("/v1/rbxm/decode", post(decode_bytes))
        .route("/v1/rbxm/decode-base64", post(decode_base64))
        .route("/v1/rbxm/validate", post(validate_bytes))
        .route("/v1/rbxm/validate-base64", post(validate_base64))
        .with_state(AppState);
    let listener = TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind port 3000");
    tracing::info!("RBXM API listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server failed");
}
