//! This single crate serves two very different builds:
//!   - native, with the "server" feature on (the default) -> the HTTP server binary (src/main.rs)
//!   - wasm32, built with `--no-default-features` (see .github/workflows/wasm.yml) -> a
//!     standalone library with no server, no network: just encode/decode/schema as plain
//!     function calls. That's why "server" is NOT required for anything in this file.

pub mod codec;
pub mod convert;
pub mod schema;
pub mod wire;

use base64::Engine;
use wasm_bindgen::prelude::*;
use wire::Tree;

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Call once, right after `init()` in JS, for readable panic messages in the browser console.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

/// JSON tree (same shape POST /encode expects) -> raw .rbxm bytes.
#[wasm_bindgen]
pub fn encode(tree_json: &str) -> Result<Vec<u8>, JsValue> {
    let tree: Tree = serde_json::from_str(tree_json).map_err(js_err)?;
    codec::encode(&tree).map_err(js_err)
}

/// Raw .rbxm bytes -> JSON tree string (same shape POST /decode returns).
#[wasm_bindgen]
pub fn decode(rbxm_bytes: &[u8]) -> Result<String, JsValue> {
    let tree = codec::decode(rbxm_bytes).map_err(js_err)?;
    serde_json::to_string(&tree).map_err(js_err)
}

/// Same as `encode`, but returns a base64 string instead of raw bytes — matching the
/// server's `?b64=1` mode. Useful when the bytes need to pass through something
/// text-only (JSON, a text field, copy-paste) rather than staying binary.
#[wasm_bindgen(js_name = encodeB64)]
pub fn encode_b64(tree_json: &str) -> Result<String, JsValue> {
    let bytes = encode(tree_json)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
 
/// Same as `decode`, but takes a base64 string instead of raw bytes.
#[wasm_bindgen(js_name = decodeB64)]
pub fn decode_b64(rbxm_b64: &str) -> Result<String, JsValue> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(rbxm_b64.trim())
        .map_err(js_err)?;
    decode(&bytes)
}

/// Same data GET /schema/:class returns, as a JSON string.
#[wasm_bindgen]
pub fn schema(class_name: &str) -> Result<String, JsValue> {
    let s = schema::class_schema(class_name)
        .ok_or_else(|| JsValue::from_str(&format!("unknown class {class_name}")))?;
    serde_json::to_string(&s).map_err(js_err)
}
