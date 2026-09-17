use std::{collections::HashMap, io::Cursor, str::FromStr};

use rbx_dom_weak::{types::{Color3, Ref, UDim, UDim2, Variant, Vector2, Vector3}, InstanceBuilder, WeakDom};
use serde_json::{json, Value};
use thiserror::Error;

use crate::models::{EncodeRequest, InstanceDoc, PropertyValue};

#[derive(Debug, Error)]
pub enum CompatError {
    #[error("invalid input: {0}")]
    Input(String),
    #[error("unsupported property type `{0}`")]
    UnsupportedType(String),
    #[error("binary codec error: {0}")]
    Binary(String),
}

fn parse_ref(value: &str) -> Result<Ref, CompatError> {
    Ref::from_str(value).map_err(|_| CompatError::Input(format!("invalid referent `{value}`")))
}

fn number(value: &Value) -> Result<f64, CompatError> {
    value.as_f64().ok_or_else(|| CompatError::Input("expected a number".into()))
}

fn vec_numbers(value: &Value, length: usize) -> Result<Vec<f64>, CompatError> {
    let values = value.as_array().ok_or_else(|| CompatError::Input("expected an array".into()))?;
    if values.len() != length { return Err(CompatError::Input(format!("expected {length} numbers"))); }
    values.iter().map(number).collect()
}

fn to_variant(property: &PropertyValue, refs: &HashMap<String, Ref>) -> Result<Variant, CompatError> {
    let kind = property.value_type.to_ascii_lowercase();
    Ok(match kind.as_str() {
        "string" | "contentid" | "content" => Variant::String(property.value.as_str().ok_or_else(|| CompatError::Input("expected a string".into()))?.to_owned()),
        "bool" | "boolean" => Variant::Bool(property.value.as_bool().ok_or_else(|| CompatError::Input("expected a boolean".into()))?),
        "int32" | "int" => Variant::Int32(number(&property.value)? as i32),
        "int64" => Variant::Int64(number(&property.value)? as i64),
        "float32" | "float" => Variant::Float32(number(&property.value)? as f32),
        "float64" | "double" => Variant::Float64(number(&property.value)?),
        "vector2" => { let v = vec_numbers(&property.value, 2)?; Variant::Vector2(Vector2::new(v[0] as f32, v[1] as f32)) },
        "vector3" => { let v = vec_numbers(&property.value, 3)?; Variant::Vector3(Vector3::new(v[0] as f32, v[1] as f32, v[2] as f32)) },
        "color3" => { let v = vec_numbers(&property.value, 3)?; Variant::Color3(Color3::new(v[0] as f32, v[1] as f32, v[2] as f32)) },
        "udim" => { let v = vec_numbers(&property.value, 2)?; Variant::UDim(UDim::new(v[0] as f32, v[1] as i32)) },
        "udim2" => { let v = vec_numbers(&property.value, 4)?; Variant::UDim2(UDim2::new(v[0] as f32, v[1] as i32, v[2] as f32, v[3] as i32)) },
        "ref" | "instance" | "instanceref" => {
            let id = property.value.as_str().ok_or_else(|| CompatError::Input("expected a referent string".into()))?;
            Variant::Ref(*refs.get(id).ok_or_else(|| CompatError::Input(format!("unknown referenced instance `{id}`")))?)
        }
        other => return Err(CompatError::UnsupportedType(other.into())),
    })
}

fn validate_request(request: &EncodeRequest) -> Result<(), CompatError> {
    if request.instances.is_empty() { return Err(CompatError::Input("instances cannot be empty".into())); }
    let mut seen = HashMap::new();
    for instance in &request.instances {
        if instance.referent.is_empty() { return Err(CompatError::Input("referents cannot be empty".into())); }
        if seen.insert(instance.referent.clone(), ()).is_some() { return Err(CompatError::Input(format!("duplicate referent `{}`", instance.referent))); }
        if instance.class_name.is_empty() { return Err(CompatError::Input("class_name cannot be empty".into())); }
    }
    for instance in &request.instances {
        if let Some(parent) = &instance.parent {
            if !seen.contains_key(parent) { return Err(CompatError::Input(format!("missing parent `{parent}`"))); }
        }
    }
    Ok(())
}

pub fn encode(request: &EncodeRequest) -> Result<Vec<u8>, CompatError> {
    validate_request(request)?;
    let refs: HashMap<String, Ref> = request.instances.iter().map(|i| (i.referent.clone(), parse_ref(&i.referent))).collect::<Result<_, _>>()?;
    let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let data_model = dom.root_ref();
    let mut inserted = HashMap::new();
    let mut remaining: Vec<&InstanceDoc> = request.instances.iter().collect();

    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for instance in remaining {
            let parent = match &instance.parent {
                None => data_model,
                Some(id) => match inserted.get(id) { Some(value) => *value, None => { next.push(instance); continue; } },
            };
            let mut builder = InstanceBuilder::new(instance.class_name.clone())
                .with_name(instance.name.clone())
                .with_referent(*refs.get(&instance.referent).unwrap());
            for (name, property) in &instance.properties { builder = builder.with_property(name.clone(), to_variant(property, &refs)?); }
            let actual = dom.insert(parent, builder);
            inserted.insert(instance.referent.clone(), actual);
        }
        if next.len() == before { return Err(CompatError::Input("parent graph contains a cycle".into())); }
        remaining = next;
    }
    let mut output = Vec::new();
    rbx_binary::to_writer(&mut output, &dom, &[data_model]).map_err(|e| CompatError::Binary(e.to_string()))?;
    Ok(output)
}

fn variant_json(value: &Variant) -> (String, Value) {
    match value {
        Variant::String(v) => ("String".into(), json!(v)),
        Variant::Bool(v) => ("Bool".into(), json!(v)),
        Variant::Int32(v) => ("Int32".into(), json!(v)),
        Variant::Int64(v) => ("Int64".into(), json!(v)),
        Variant::Float32(v) => ("Float32".into(), json!(v)),
        Variant::Float64(v) => ("Float64".into(), json!(v)),
        Variant::Ref(v) => ("Ref".into(), json!(v.to_string())),
        other => (format!("{:?}", other.ty()), Value::Null),
    }
}

pub fn decode(bytes: &[u8]) -> Result<Value, CompatError> {
    let dom = rbx_binary::from_reader(Cursor::new(bytes)).map_err(|e| CompatError::Binary(e.to_string()))?;
    let root = dom.root_ref();
    let mut instances = Vec::new();
    for instance in dom.descendants_of(root).skip(1) {
        let parent = if instance.parent() == root { None } else { dom.get_by_ref(instance.parent()).map(|p| p.referent().to_string()) };
        let properties = instance.properties.iter().map(|(name, value)| {
            let (value_type, value) = variant_json(value);
            (name.to_string(), json!({"value_type": value_type, "value": value}))
        }).collect::<serde_json::Map<_, _>>();
        instances.push(json!({"referent": instance.referent().to_string(), "class_name": instance.class.to_string(), "name": instance.name, "parent": parent, "properties": properties}));
    }
    Ok(json!({"instances": instances}))
}

pub fn validate(bytes: &[u8]) -> Result<(), CompatError> {
    let _ = rbx_binary::from_reader(Cursor::new(bytes)).map_err(|e| CompatError::Binary(e.to_string()))?;
    Ok(())
}
