mod codec;
mod convert;
mod schema;
mod wire;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::Deserialize;

use codec::Limits;

struct AppState {
    api_key: Option<String>,
    limits: Limits,
}

type Shared = Arc<AppState>;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        api_key: std::env::var("RBXM_API_KEY").ok().filter(|k| !k.is_empty()),
        limits: Limits {
            max_instances: env_usize("MAX_INSTANCES", 50_000),
            max_depth: env_usize("MAX_DEPTH", 256),
        },
    });
    if state.api_key.is_none() {
        eprintln!("WARNING: RBXM_API_KEY is not set — the API is unauthenticated!");
    }

    let max_body = env_usize("MAX_BODY_BYTES", 8 * 1024 * 1024);
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/schema/:class", get(schema_route))
        .route("/schemas", get(schemas_route))
        .route("/encode", post(encode_route))
        .route("/decode", post(decode_route))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state);

    let port = env_usize("PORT", 8080) as u16;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("rbxm-api listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ------------------------------------------------------------------ helpers

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

fn bad_request(e: impl std::fmt::Display) -> ApiError {
    // {:#} prints the whole anyhow context chain.
    ApiError(StatusCode::BAD_REQUEST, format!("{e:#}"))
}

fn auth(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = &state.api_key else { return Ok(()) };
    let given = headers.get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or("");
    // Constant-time-ish comparison.
    let ok = given.len() == expected.len()
        && given.bytes().zip(expected.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0;
    if ok { Ok(()) } else { Err(ApiError(StatusCode::UNAUTHORIZED, "bad or missing x-api-key".into())) }
}

#[derive(Deserialize)]
struct FormatQuery {
    /// `?b64=1` — request/response body is base64 text instead of raw bytes.
    #[serde(default)]
    b64: Option<String>,
}

impl FormatQuery {
    fn is_b64(&self) -> bool {
        matches!(self.b64.as_deref(), Some("1" | "true"))
    }
}

// ------------------------------------------------------------------ routes

async fn schema_route(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(class): Path<String>,
) -> Result<Response, ApiError> {
    auth(&state, &headers)?;
    match schema::class_schema(&class) {
        Some(s) => Ok(Json(s).into_response()),
        None => Err(ApiError(StatusCode::NOT_FOUND, format!("unknown class {class}"))),
    }
}

#[derive(Deserialize)]
struct SchemasQuery {
    /// Comma-separated class names, e.g. `?classes=Part,Model,Script`.
    classes: String,
}

/// Batch version of /schema/:class — one HTTP request instead of one per class
/// (HttpService is limited to 500 requests/minute per game server).
async fn schemas_route(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<SchemasQuery>,
) -> Result<Response, ApiError> {
    auth(&state, &headers)?;
    let mut out = Vec::new();
    for class in q.classes.split(',').map(str::trim).filter(|c| !c.is_empty()).take(200) {
        if let Some(s) = schema::class_schema(class) {
            out.push(s);
        }
    }
    Ok(Json(serde_json::json!({ "schemas": out })).into_response())
}

async fn encode_route(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<FormatQuery>,
    Json(tree): Json<wire::Tree>,
) -> Result<Response, ApiError> {
    auth(&state, &headers)?;
    let st = state.clone();
    let bytes = tokio::task::spawn_blocking(move || codec::encode(&tree, &st.limits))
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(bad_request)?;

    if q.is_b64() {
        let text = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(([(header::CONTENT_TYPE, "text/plain")], text).into_response())
    } else {
        Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
    }
}

async fn decode_route(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(q): Query<FormatQuery>,
    body: Bytes,
) -> Result<Response, ApiError> {
    auth(&state, &headers)?;
    let raw: Vec<u8> = if q.is_b64() {
        let text = std::str::from_utf8(&body).map_err(bad_request)?;
        base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(bad_request)?
    } else {
        body.to_vec()
    };
    let st = state.clone();
    let tree = tokio::task::spawn_blocking(move || codec::decode(&raw, &st.limits))
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(bad_request)?;
    Ok(Json(tree).into_response())
}
