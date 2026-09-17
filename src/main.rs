mod compat;
mod models;
mod packet;

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

use crate::models::{
    DecodeRequest, DecodeResponse, EncodeRequest, EncodeResponse, ErrorResponse, HealthResponse,
    ValidateRequest, ValidateResponse,
};

#[derive(Clone, Default)]
struct AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

fn error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            code: code.into(),
            message: message.into(),
        }),
    )
}

fn binary_response(bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(bytes.into())
        .expect("valid binary response")
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "rbxm-api".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Production endpoint: RBXI instance packet bytes -> actual RBXM bytes.
async fn encode_bytes(
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let output = packet::packet_to_rbxm(&body)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "packet_error", error.to_string()))?;
    Ok(binary_response(output))
}

/// Production endpoint: actual RBXM bytes -> RBXI instance packet bytes.
async fn decode_bytes(
    body: Bytes,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let output = packet::rbxm_to_packet(&body)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "decode_error", error.to_string()))?;
    Ok(binary_response(output))
}

async fn encode_json(Json(request): Json<EncodeRequest>) -> ApiResult<EncodeResponse> {
    let bytes = compat::encode(&request)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "encode_error", error.to_string()))?;
    Ok(Json(EncodeResponse {
        format: request.format,
        encoding: "base64".into(),
        data: STANDARD.encode(bytes),
        note: "Debug JSON adapter; production uses /v1/rbxm/encode with RBXI bytes.".into(),
    }))
}

async fn decode_base64(Json(request): Json<DecodeRequest>) -> ApiResult<DecodeResponse> {
    let bytes = STANDARD
        .decode(request.data)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "base64_error", error.to_string()))?;
    let data = compat::decode(&bytes)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "decode_error", error.to_string()))?;
    Ok(Json(DecodeResponse {
        format: request.format,
        data,
        note: "Debug JSON decoder; production uses /v1/rbxm/decode with RBXM bytes.".into(),
    }))
}

async fn validate_bytes(body: Bytes) -> ApiResult<ValidateResponse> {
    let result = rbx_binary::from_reader(std::io::Cursor::new(&body));
    match result {
        Ok(_) => Ok(Json(ValidateResponse {
            ok: true,
            format: "rbxm".into(),
            issues: vec![],
            note: "Valid RBXM/RBXL binary document.".into(),
        })),
        Err(error) => Ok(Json(ValidateResponse {
            ok: false,
            format: "rbxm".into(),
            issues: vec![error.to_string()],
            note: "rbx_binary rejected the document.".into(),
        })),
    }
}

async fn validate_base64(Json(request): Json<ValidateRequest>) -> ApiResult<ValidateResponse> {
    let bytes = STANDARD
        .decode(request.data)
        .map_err(|error| error(StatusCode::BAD_REQUEST, "base64_error", error.to_string()))?;
    validate_bytes(Bytes::from(bytes)).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/rbxm/encode", post(encode_bytes))
        .route("/v1/rbxm/decode", post(decode_bytes))
        .route("/v1/rbxm/validate", post(validate_bytes))
        .route("/v1/debug/encode-json", post(encode_json))
        .route("/v1/debug/decode-base64", post(decode_base64))
        .route("/v1/debug/validate-base64", post(validate_base64))
        .with_state(AppState);

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");
    tracing::info!("RBXM API listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server failed");
}
