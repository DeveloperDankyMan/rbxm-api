//! wasm-bindgen bindings for rbxm_core — the encode/decode logic with no server,
//! no network, no filesystem. Runs entirely in-process: feed it a JSON tree, get
//! .rbxm bytes back (or vice versa). Built with `wasm-pack build --target web`
//! (see ../.github/workflows/wasm.yml), which produces `rbxm_wasm.js` + `rbxm_wasm_bg.wasm`.
//!
//! JS usage (ES module, --target web):
//!     import init, { encode, decode, schema } from './rbxm_wasm.js';
//!     await init();
//!     const bytes = encode(JSON.stringify(tree));   // Uint8Array
//!     const treeJson = decode(bytes);                // string, JSON.parse it
//!
//! Python usage (via the `wasmtime` package, NOT this file's JS glue — see README):
//!     the exported `encode` / `decode` / `schema` functions below are plain
//!     wasm functions once compiled; wasmtime calls them directly without the
//!     JS glue file, which is JS-specific and not usable from Python.

use rbxm_core::{codec, schema as schema_mod, wire::Tree};
use wasm_bindgen::prelude::*;

fn js_err(e: impl std::fmt::Display) -> JsValue {
    // {:#} would be nicer (full anyhow chain) but Display is enough here and avoids
    // requiring the caller to know anyhow's formatting.
    JsValue::from_str(&e.to_string())
}

/// Call once, right after `init()`, to get readable panic messages in the browser console
/// instead of an opaque "unreachable" trap. Safe to call more than once.
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
/// Returns an error if the class is unknown to rbx_reflection_database.
#[wasm_bindgen]
pub fn schema(class_name: &str) -> Result<String, JsValue> {
    let s = schema_mod::class_schema(class_name)
        .ok_or_else(|| JsValue::from_str(&format!("unknown class {class_name}")))?;
    serde_json::to_string(&s).map_err(js_err)
}
