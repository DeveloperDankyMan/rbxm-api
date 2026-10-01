// src/lib.rs
pub mod codec; // move your rbx_dom encode/decode logic into src/codec.rs

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn rbxm_to_json(bytes: &[u8]) -> Result<String, JsError> {
    codec::decode_to_json(bytes).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn json_to_rbxm(json: &str) -> Result<Vec<u8>, JsError> {
    codec::encode_from_json(json).map_err(|e| JsError::new(&e.to_string()))
}
