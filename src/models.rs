use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceDoc {
    pub referent: String,
    pub class_name: String,
    pub name: String,
    pub parent: Option<String>,
    pub properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PropertyValue {
    pub value_type: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncodeRequest {
    pub format: String,
    pub instances: Vec<InstanceDoc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncodeResponse {
    pub format: String,
    pub encoding: String,
    pub data: String,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecodeRequest {
    pub format: String,
    pub encoding: String,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecodeResponse {
    pub format: String,
    pub data: serde_json::Value,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidateRequest {
    pub format: String,
    pub encoding: String,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidateResponse {
    pub ok: bool,
    pub format: String,
    pub issues: Vec<String>,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
    pub version: String,
}
