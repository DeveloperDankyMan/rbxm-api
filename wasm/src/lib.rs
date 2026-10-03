use rbxm_core::{codec, schema as schema_mod, wire::Tree};
use wasm_bindgen::prelude::*;

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn encode(tree_json: &str) -> Result<Vec<u8>, JsValue> {
    let tree: Tree = serde_json::from_str(tree_json).map_err(js_err)?;
    codec::encode(&tree).map_err(js_err)
}

#[wasm_bindgen]
pub fn decode(rbxm_bytes: &[u8]) -> Result<String, JsValue> {
    let tree = codec::decode(rbxm_bytes).map_err(js_err)?;
    serde_json::to_string(&tree).map_err(js_err)
}

#[wasm_bindgen]
pub fn schema(class_name: &str) -> Result<String, JsValue> {
    let s = schema_mod::class_schema(class_name)
        .ok_or_else(|| JsValue::from_str(&format!("unknown class {class_name}")))?;
    serde_json::to_string(&s).map_err(js_err)
}
