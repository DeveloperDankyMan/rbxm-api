pub mod codec;
pub mod convert;
pub mod wire;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn rbxm_to_json(bytes: &[u8]) -> Result<String, JsError> {
    let tree = codec::decode(bytes).map_err(|e| JsError::new(&format!("{e:#}")))?;
    serde_json::to_string(&tree).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn json_to_rbxm(json: &str) -> Result<Vec<u8>, JsError> {
    let tree: wire::Tree =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    codec::encode(&tree).map_err(|e| JsError::new(&format!("{e:#}")))
}
