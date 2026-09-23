//! GET /schema/:class — tells the Luau client which properties to read and how they are typed,
//! straight from rbx_reflection_database, so the client needs no hardcoded property lists.

use rbx_reflection::{DataType, PropertyKind, PropertySerialization, Scriptability};
use serde::Serialize;

use crate::convert::type_name;

#[derive(Serialize)]
pub struct PropInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
    /// Enum name (for `Enum` properties), e.g. "Material".
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_name: Option<String>,
}

#[derive(Serialize)]
pub struct ClassSchema {
    pub class: String,
    pub properties: Vec<PropInfo>,
}

pub fn class_schema(class: &str) -> Option<ClassSchema> {
    let db = rbx_reflection_database::get().ok()?;
    let start = db.classes.get(class)?;
    let mut props: Vec<PropInfo> = Vec::new();

    for cls in db.superclasses_iter(start) {
        for desc in cls.properties.values() {
            // Only canonical properties that are written to files and that a script can read.
            let PropertyKind::Canonical { serialization } = &desc.kind else { continue };
            if !matches!(serialization, PropertySerialization::Serializes | PropertySerialization::SerializesAs(_)) {
                continue;
            }
            if !matches!(desc.scriptability, Scriptability::Read | Scriptability::ReadWrite | Scriptability::Custom) {
                continue;
            }
            if matches!(desc.name, "Name" | "Parent" | "ClassName") {
                continue;
            }
            let (ty, enum_name) = match &desc.data_type {
                DataType::Value(t) => match type_name(*t) {
                    Some(n) => (n, None),
                    None => continue,
                },
                DataType::Enum(e) => ("Enum", Some((*e).to_owned())),
                _ => continue,
            };
            if props.iter().any(|p| p.name == desc.name) {
                continue; // subclass already provided it
            }
            props.push(PropInfo { name: desc.name.to_owned(), ty, enum_name });
        }
    }
    props.sort_by(|a, b| a.name.cmp(&b.name));
    Some(ClassSchema { class: class.to_owned(), properties: props })
}
