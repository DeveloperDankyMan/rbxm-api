use std::io::{self, Cursor, Read};

use rbx_dom_weak::types::Variant;

use crate::{
    compat,
    models::{EncodeRequest, InstanceDoc, PropertyValue},
};

const MAGIC: &[u8; 4] = b"RBXI";
const VERSION: u8 = 1;

#[derive(Debug)]
pub enum PacketError {
    Invalid(String),
    Io(io::Error),
}

impl From<io::Error> for PacketError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(value) => write!(f, "{value}"),
            Self::Io(value) => write!(f, "{value}"),
        }
    }
}

impl std::error::Error for PacketError {}

fn read_u32(reader: &mut Cursor<&[u8]>) -> Result<u32, PacketError> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_string(reader: &mut Cursor<&[u8]>) -> Result<String, PacketError> {
    let length = read_u32(reader)? as usize;
    if length > 1024 * 1024 {
        return Err(PacketError::Invalid("string is too large".into()));
    }

    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| PacketError::Invalid("invalid UTF-8 string".into()))
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), PacketError> {
    let bytes = value.as_bytes();
    let length = u32::try_from(bytes.len())
        .map_err(|_| PacketError::Invalid("string is too large".into()))?;
    output.extend(length.to_le_bytes());
    output.extend(bytes);
    Ok(())
}

fn encode_property_value(value: &serde_json::Value, kind: &str) -> Result<Vec<u8>, PacketError> {
    match kind {
        "String" | "Ref" => {
            let mut output = Vec::new();
            write_string(
                &mut output,
                value
                    .as_str()
                    .ok_or_else(|| PacketError::Invalid(format!("{kind} value required")))?,
            )?;
            Ok(output)
        }
        "Bool" => Ok(vec![
            value
                .as_bool()
                .ok_or_else(|| PacketError::Invalid("Bool value required".into()))?
                as u8,
        ]),
        "Int32" => Ok((value
            .as_i64()
            .ok_or_else(|| PacketError::Invalid("Int32 value required".into()))?
            as i32)
            .to_le_bytes()
            .to_vec()),
        "Int64" => Ok(value
            .as_i64()
            .ok_or_else(|| PacketError::Invalid("Int64 value required".into()))?
            .to_le_bytes()
            .to_vec()),
        "Float32" => Ok((value
            .as_f64()
            .ok_or_else(|| PacketError::Invalid("Float32 value required".into()))?
            as f32)
            .to_le_bytes()
            .to_vec()),
        "Float64" => Ok(value
            .as_f64()
            .ok_or_else(|| PacketError::Invalid("Float64 value required".into()))?
            .to_le_bytes()
            .to_vec()),
        "Vector2" | "Vector3" | "Color3" => {
            let values = value
                .as_array()
                .ok_or_else(|| PacketError::Invalid(format!("{kind} value required")))?;
            let expected = if kind == "Vector2" { 2 } else { 3 };
            if values.len() != expected {
                return Err(PacketError::Invalid(format!("{kind} requires {expected} numbers")));
            }
            let mut output = Vec::with_capacity(expected * 4);
            for number in values {
                output.extend(
                    (number
                        .as_f64()
                        .ok_or_else(|| PacketError::Invalid(format!("{kind} numbers required")))?
                        as f32)
                        .to_le_bytes(),
                );
            }
            Ok(output)
        }
        other => Err(PacketError::Invalid(format!(
            "unsupported packet property type `{other}`"
        ))),
    }
}

pub fn decode(bytes: &[u8]) -> Result<EncodeRequest, PacketError> {
    let mut reader = Cursor::new(bytes);
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(PacketError::Invalid("invalid RBXI magic".into()));
    }

    let mut version = [0; 1];
    reader.read_exact(&mut version)?;
    if version[0] != VERSION {
        return Err(PacketError::Invalid("unsupported RBXI version".into()));
    }

    let count = read_u32(&mut reader)? as usize;
    if count > 100_000 {
        return Err(PacketError::Invalid("too many instances".into()));
    }

    let mut instances = Vec::with_capacity(count);
    for _ in 0..count {
        let referent = read_string(&mut reader)?;
        let parent = read_string(&mut reader)?;
        let class_name = read_string(&mut reader)?;
        let name = read_string(&mut reader)?;
        let property_count = read_u32(&mut reader)? as usize;
        if property_count > 100_000 {
            return Err(PacketError::Invalid("too many properties".into()));
        }

        let mut properties = std::collections::BTreeMap::new();
        for _ in 0..property_count {
            let property_name = read_string(&mut reader)?;
            let kind = read_string(&mut reader)?;
            let value = match kind.as_str() {
                "String" | "Ref" => serde_json::Value::String(read_string(&mut reader)?),
                "Bool" => {
                    let mut byte = [0];
                    reader.read_exact(&mut byte)?;
                    if byte[0] > 1 {
                        return Err(PacketError::Invalid("invalid Bool value".into()));
                    }
                    serde_json::Value::Bool(byte[0] != 0)
                }
                "Int32" => {
                    let mut bytes = [0; 4];
                    reader.read_exact(&mut bytes)?;
                    serde_json::json!(i32::from_le_bytes(bytes))
                }
                "Int64" => {
                    let mut bytes = [0; 8];
                    reader.read_exact(&mut bytes)?;
                    serde_json::json!(i64::from_le_bytes(bytes))
                }
                "Float32" => {
                    let mut bytes = [0; 4];
                    reader.read_exact(&mut bytes)?;
                    serde_json::json!(f32::from_le_bytes(bytes))
                }
                "Float64" => {
                    let mut bytes = [0; 8];
                    reader.read_exact(&mut bytes)?;
                    serde_json::json!(f64::from_le_bytes(bytes))
                }
                "Vector2" | "Vector3" | "Color3" => {
                    let count = if kind == "Vector2" { 2 } else { 3 };
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        let mut bytes = [0; 4];
                        reader.read_exact(&mut bytes)?;
                        values.push(serde_json::json!(f32::from_le_bytes(bytes)));
                    }
                    serde_json::Value::Array(values)
                }
                other => {
                    return Err(PacketError::Invalid(format!(
                        "unsupported packet property type `{other}`"
                    )))
                }
            };

            properties.insert(
                property_name,
                PropertyValue {
                    value_type: kind,
                    value,
                },
            );
        }

        instances.push(InstanceDoc {
            referent,
            class_name,
            name,
            parent: if parent.is_empty() { None } else { Some(parent) },
            properties,
        });
    }

    if reader.position() != bytes.len() as u64 {
        return Err(PacketError::Invalid("trailing bytes after RBXI packet".into()));
    }

    Ok(EncodeRequest {
        format: "rbxm".into(),
        instances,
    })
}

pub fn encode(request: &EncodeRequest) -> Result<Vec<u8>, PacketError> {
    let mut output = Vec::new();
    output.extend(MAGIC);
    output.push(VERSION);
    output.extend(
        u32::try_from(request.instances.len())
            .map_err(|_| PacketError::Invalid("too many instances".into()))?
            .to_le_bytes(),
    );

    for instance in &request.instances {
        write_string(&mut output, &instance.referent)?;
        write_string(&mut output, instance.parent.as_deref().unwrap_or(""))?;
        write_string(&mut output, &instance.class_name)?;
        write_string(&mut output, &instance.name)?;
        output.extend(
            u32::try_from(instance.properties.len())
                .map_err(|_| PacketError::Invalid("too many properties".into()))?
                .to_le_bytes(),
        );

        for (name, property) in &instance.properties {
            write_string(&mut output, name)?;
            write_string(&mut output, &property.value_type)?;
            output.extend(encode_property_value(&property.value, &property.value_type)?);
        }
    }

    Ok(output)
}

pub fn packet_to_rbxm(bytes: &[u8]) -> Result<Vec<u8>, PacketError> {
    let request = decode(bytes)?;
    compat::encode(&request).map_err(|error| PacketError::Invalid(error.to_string()))
}

pub fn rbxm_to_packet(bytes: &[u8]) -> Result<Vec<u8>, PacketError> {
    let dom = rbx_binary::from_reader(Cursor::new(bytes))
        .map_err(|error| PacketError::Invalid(error.to_string()))?;
    let mut request = EncodeRequest {
        format: "rbxm".into(),
        instances: Vec::new(),
    };

    for instance in dom.descendants().skip(1) {
        let parent = if instance.parent() == dom.root_ref() {
            None
        } else {
            dom.get_by_ref(instance.parent())
                .map(|parent| parent.referent().to_string())
        };
        let mut properties = std::collections::BTreeMap::new();

        for (name, value) in &instance.properties {
            let converted = match value {
                Variant::String(value) => ("String", serde_json::json!(value)),
                Variant::Bool(value) => ("Bool", serde_json::json!(value)),
                Variant::Int32(value) => ("Int32", serde_json::json!(value)),
                Variant::Int64(value) => ("Int64", serde_json::json!(value)),
                Variant::Float32(value) => ("Float32", serde_json::json!(value)),
                Variant::Float64(value) => ("Float64", serde_json::json!(value)),
                Variant::Vector2(value) => ("Vector2", serde_json::json!([value.x, value.y])),
                Variant::Vector3(value) => (
                    "Vector3",
                    serde_json::json!([value.x, value.y, value.z]),
                ),
                Variant::Color3(value) => (
                    "Color3",
                    serde_json::json!([value.r, value.g, value.b]),
                ),
                Variant::Ref(value) => ("Ref", serde_json::json!(value.to_string())),
                _ => continue,
            };
            properties.insert(
                name.to_string(),
                PropertyValue {
                    value_type: converted.0.into(),
                    value: converted.1,
                },
            );
        }

        request.instances.push(InstanceDoc {
            referent: instance.referent().to_string(),
            class_name: instance.class.to_string(),
            name: instance.name.clone(),
            parent,
            properties,
        });
    }

    encode(&request)
}
