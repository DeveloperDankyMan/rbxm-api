mod models;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::models::{
    EncodeRequest, EncodeResponse, ErrorResponse, HealthResponse, ValidateRequest, ValidateResponse,
};

#[derive(Clone, Default)]
struct AppState {}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "rbxm-api".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn encode_rbxm(
    State(_state): State<AppState>,
    Json(payload): Json<EncodeRequest>,
) -> Result<Json<EncodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.format.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "invalid_format".to_string(),
                message: "format is required".to_string(),
            }),
        ));
    }

    // This is intentionally a scaffold. The real implementation should be wired
    // to a public-format RBXM encoder (e.g. a compatibility layer around Rojo's
    // rbx-dom model or a maintained public format encoder).
    let payload_bytes = serde_json::to_vec(&payload).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "serialize_error".to_string(),
                message: err.to_string(),
            }),
        )
    })?;

    let encoded = base64::encode(payload_bytes);

    Ok(Json(EncodeResponse {
        format: payload.format.clone(),
        encoding: "base64".to_string(),
        data: encoded,
        note: "public-format compatibility scaffold; replace with actual RBXM encoder implementation".to_string(),
    }))
}

async fn decode_rbxm(
    State(_state): State<AppState>,
    Json(payload): Json<models::DecodeRequest>,
) -> Result<Json<models::DecodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.data.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "empty_payload".to_string(),
                message: "data is required".to_string(),
            }),
        ));
    }

    let decoded = base64::decode(&payload.data).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "invalid_base64".to_string(),
                message: err.to_string(),
            }),
        )
    })?;

    let parsed: serde_json::Value = serde_json::from_slice(&decoded).unwrap_or(json!({
        "status": "decoded-but-not-parsed",
        "note": "this scaffold does not yet implement the public RBXM decoder"
    }));

    Ok(Json(models::DecodeResponse {
        format: payload.format.clone(),
        data: parsed,
        note: "public-format compatibility scaffold; replace with actual RBXM decoder implementation".to_string(),
    }))
}

async fn validate_rbxm(
    State(_state): State<AppState>,
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.data.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "empty_payload".to_string(),
                message: "data is required".to_string(),
            }),
        ));
    }

    let ok = payload.format == "rbxm" || payload.format == "rbxmx" || payload.format == "rbxl";

    Ok(Json(ValidateResponse {
        ok,
        format: payload.format,
        issues: if ok {
            vec![]
        } else {
            vec!["unsupported format".to_string()]
        },
        note: "public-format validation scaffold; replace with exact chunk validation logic".to_string(),
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/rbxm/encode", post(encode_rbxm))
        .route("/v1/rbxm/decode", post(decode_rbxm))
        .route("/v1/rbxm/validate", post(validate_rbxm))
        .with_state(AppState::default());

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");

    tracing::info!("RBXM compatibility API listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.expect("server failed");
}
