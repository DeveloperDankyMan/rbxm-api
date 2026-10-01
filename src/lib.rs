pub mod codec;
pub mod convert;
pub mod schema;
pub mod wire;

use base64::Engine;
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

/// Encode instances (JSON tree) to .rbxm bytes, then base64 encode for transport
#[wasm_bindgen]
pub fn encode_to_base64(json: &str) -> Result<String, JsError> {
    let tree: wire::Tree =
        serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
    let bytes = codec::encode(&tree).map_err(|e| JsError::new(&format!("{e:#}")))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(b64)
}

/// Decode base64 string to .rbxm bytes, then deserialize to JSON tree
#[wasm_bindgen]
pub fn decode_from_base64(b64: &str) -> Result<String, JsError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| JsError::new(&format!("base64 decode error: {e}")))?;
    let tree = codec::decode(&bytes).map_err(|e| JsError::new(&format!("{e:#}")))?;
    serde_json::to_string(&tree).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn class_schema_json(class: &str) -> Result<String, JsError> {
    match schema::class_schema(class) {
        Some(s) => serde_json::to_string(&s).map_err(|e| JsError::new(&e.to_string())),
        None => Err(JsError::new(&format!("unknown class {class}"))),
    }
}

#[wasm_bindgen]
pub fn class_schemas_json(classes: &str) -> Result<String, JsError> {
    let mut out = Vec::new();
    for class in classes.split(',').map(str::trim).filter(|c| !c.is_empty()).take(200) {
        if let Some(s) = schema::class_schema(class) {
            out.push(s);
        }
    }
    let result = serde_json::json!({ "schemas": out });
    serde_json::to_string(&result).map_err(|e| JsError::new(&e.to_string()))
}
