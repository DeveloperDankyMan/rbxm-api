pub mod codec;
pub mod convert;
pub mod schema;
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
