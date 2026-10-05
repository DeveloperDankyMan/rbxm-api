//! wasm-bindgen bindings for rbxm_core (the "rbxm-api" package at ../, built with
//! default-features = false so axum/tokio never enter this build). Pure encode/decode/
//! schema logic, no server, no network. Built by ../.github/workflows/wasm.yml via:
//!   wasm-pack build --target web --release --out-dir pkg   (run from inside wasm/)
//! which produces wasm/pkg/rbxm_core.js + wasm/pkg/rbxm_core_bg.wasm.
//!
//! JS usage (ES module, --target web):
//!     import init, { encode, decode, schema } from './rbxm_core.js';
//!     await init();
//!     const bytes = encode(JSON.stringify(tree));   // Uint8Array
//!     const treeJson = decode(bytes);                // string, JSON.parse it

use rbxm_core::{codec, schema as schema_mod, wire::Tree};
use wasm_bindgen::prelude::*;

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// JSON tree (same shape the server's POST /encode expects) -> raw .rbxm bytes.
#[wasm_bindgen]
pub fn encode(tree_json: &str) -> Result<Vec<u8>, JsValue> {
    let tree: Tree = serde_json::from_str(tree_json).map_err(js_err)?;
    codec::encode(&tree).map_err(js_err)
}

/// Raw .rbxm bytes -> JSON tree string (same shape the server's POST /decode returns).
#[wasm_bindgen]
pub fn decode(rbxm_bytes: &[u8]) -> Result<String, JsValue> {
    let tree = codec::decode(rbxm_bytes).map_err(js_err)?;
    serde_json::to_string(&tree).map_err(js_err)
}

/// Same data the server's GET /schema/:class returns, as a JSON string.
#[wasm_bindgen]
pub fn schema(class_name: &str) -> Result<String, JsValue> {
    let s = schema_mod::class_schema(class_name)
        .ok_or_else(|| JsValue::from_str(&format!("unknown class {class_name}")))?;
    serde_json::to_string(&s).map_err(js_err)
}

/// Same data GET /schemas?classes=A,B,C returns: a comma-separated list of class names in,
/// `{"schemas": [...]}` out as a JSON string. Classes rbx_reflection_database doesn't
/// recognize are silently skipped, same as the server route.
#[wasm_bindgen]
pub fn schemas(class_names: &str) -> Result<String, JsValue> {
    let out: Vec<_> = class_names
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .filter_map(schema::class_schema)
        .collect();
    serde_json::to_string(&serde_json::json!({ "schemas": out })).map_err(js_err)
}
