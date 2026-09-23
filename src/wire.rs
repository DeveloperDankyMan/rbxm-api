//! JSON wire format shared by /encode and /decode.
//!
//! Every property value is tagged with its rbx-dom type name:
//!   { "t": "Vector3", "v": [1, 2, 3] }
//! See README.md for the shape of `v` for each type.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire {
    pub t: String,
    #[serde(default)]
    pub v: Value,
    /// Only sent by /decode, for `Enum` values: the enum's name (e.g. "Material").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceDesc {
    /// Any string that is unique inside this request. Used for parents and Ref properties.
    pub id: String,
    pub class: String,
    #[serde(default)]
    pub name: Option<String>,
    /// `null` / missing = top-level instance in the .rbxm file.
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, Wire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub instances: Vec<InstanceDesc>,
}
