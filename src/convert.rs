//! Conversion between the JSON wire values and rbx_types `Variant`s.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use rbx_types::{
    Attributes, Axes, BinaryString, BrickColor, CFrame, Color3, Color3uint8, ColorSequence,
    ColorSequenceKeypoint, Content, ContentId, ContentType, CustomPhysicalProperties, Enum, Faces,
    Font, FontStyle, FontWeight, Matrix3, NumberRange, NumberSequence, NumberSequenceKeypoint,
    PhysicalProperties, Ray, Rect, Ref, SharedString, Tags, UDim, UDim2, Variant, VariantType,
    Vector2, Vector2int16, Vector3, Vector3int16,
};
use serde_json::{json, Map, Value};

use crate::wire::Wire;

/// Maps an rbx-dom type name (as used in the wire format) to a `VariantType`.
pub fn parse_type(name: &str) -> Option<VariantType> {
    Some(match name {
        "Axes" => VariantType::Axes,
        "BinaryString" => VariantType::BinaryString,
        "Bool" => VariantType::Bool,
        "BrickColor" => VariantType::BrickColor,
        "CFrame" => VariantType::CFrame,
        "Color3" => VariantType::Color3,
        "Color3uint8" => VariantType::Color3uint8,
        "ColorSequence" => VariantType::ColorSequence,
        "Content" => VariantType::Content,
        "ContentId" => VariantType::ContentId,
        "Enum" => VariantType::Enum,
        "Faces" => VariantType::Faces,
        "Float32" => VariantType::Float32,
        "Float64" => VariantType::Float64,
        "Font" => VariantType::Font,
        "Int32" => VariantType::Int32,
        "Int64" => VariantType::Int64,
        "NumberRange" => VariantType::NumberRange,
        "NumberSequence" => VariantType::NumberSequence,
        "OptionalCFrame" => VariantType::OptionalCFrame,
        "PhysicalProperties" => VariantType::PhysicalProperties,
        "Ray" => VariantType::Ray,
        "Rect" => VariantType::Rect,
        "Ref" => VariantType::Ref,
        "SharedString" => VariantType::SharedString,
        "String" => VariantType::String,
        "Tags" => VariantType::Tags,
        "Attributes" => VariantType::Attributes,
        "UDim" => VariantType::UDim,
        "UDim2" => VariantType::UDim2,
        "Vector2" => VariantType::Vector2,
        "Vector2int16" => VariantType::Vector2int16,
        "Vector3" => VariantType::Vector3,
        "Vector3int16" => VariantType::Vector3int16,
        _ => return None,
    })
}

pub fn type_name(ty: VariantType) -> Option<&'static str> {
    Some(match ty {
        VariantType::Axes => "Axes",
        VariantType::BinaryString => "BinaryString",
        VariantType::Bool => "Bool",
        VariantType::BrickColor => "BrickColor",
        VariantType::CFrame => "CFrame",
        VariantType::Color3 => "Color3",
        VariantType::Color3uint8 => "Color3uint8",
        VariantType::ColorSequence => "ColorSequence",
        VariantType::Content => "Content",
        VariantType::ContentId => "ContentId",
        VariantType::Enum => "Enum",
        VariantType::Faces => "Faces",
        VariantType::Float32 => "Float32",
        VariantType::Float64 => "Float64",
        VariantType::Font => "Font",
        VariantType::Int32 => "Int32",
        VariantType::Int64 => "Int64",
        VariantType::NumberRange => "NumberRange",
        VariantType::NumberSequence => "NumberSequence",
        VariantType::OptionalCFrame => "OptionalCFrame",
        VariantType::PhysicalProperties => "PhysicalProperties",
        VariantType::Ray => "Ray",
        VariantType::Rect => "Rect",
        VariantType::Ref => "Ref",
        VariantType::SharedString => "SharedString",
        VariantType::String => "String",
        VariantType::Tags => "Tags",
        VariantType::Attributes => "Attributes",
        VariantType::UDim => "UDim",
        VariantType::UDim2 => "UDim2",
        VariantType::Vector2 => "Vector2",
        VariantType::Vector2int16 => "Vector2int16",
        VariantType::Vector3 => "Vector3",
        VariantType::Vector3int16 => "Vector3int16",
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn num(v: &Value) -> Result<f64> {
    v.as_f64().ok_or_else(|| anyhow!("expected a number, got {v}"))
}

fn nums(v: &Value, n: usize) -> Result<Vec<f64>> {
    let arr = v.as_array().ok_or_else(|| anyhow!("expected an array of {n} numbers"))?;
    if arr.len() != n {
        bail!("expected {n} numbers, got {}", arr.len());
    }
    arr.iter().map(num).collect()
}

fn str_of(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| anyhow!("expected a string, got {v}"))
}

fn b64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .context("invalid base64")
}

fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn v3(a: &[f64]) -> Vector3 {
    Vector3::new(a[0] as f32, a[1] as f32, a[2] as f32)
}

fn cframe_from(a: &[f64]) -> CFrame {
    // Same order as CFrame:GetComponents(): x, y, z, R00, R01, R02, R10, R11, R12, R20, R21, R22
    // (rows of the rotation matrix).
    CFrame::new(
        v3(&a[0..3]),
        Matrix3::new(v3(&a[3..6]), v3(&a[6..9]), v3(&a[9..12])),
    )
}

fn cframe_to(cf: &CFrame) -> Value {
    let m = &cf.orientation;
    json!([
        cf.position.x, cf.position.y, cf.position.z,
        m.x.x, m.x.y, m.x.z,
        m.y.x, m.y.y, m.y.z,
        m.z.x, m.z.y, m.z.z,
    ])
}

// ---------------------------------------------------------------- JSON -> Variant

/// `refs` maps request ids to the referents they were assigned.
pub fn to_variant(ty: VariantType, v: &Value, refs: &HashMap<String, Ref>) -> Result<Variant> {
    Ok(match ty {
        VariantType::Bool => Variant::Bool(v.as_bool().ok_or_else(|| anyhow!("expected bool"))?),
        VariantType::Int32 => Variant::Int32(num(v)? as i32),
        VariantType::Int64 => Variant::Int64(num(v)? as i64),
        VariantType::Float32 => Variant::Float32(num(v)? as f32),
        VariantType::Float64 => Variant::Float64(num(v)?),
        VariantType::String => Variant::String(str_of(v)?.to_owned()),
        VariantType::BinaryString => Variant::BinaryString(BinaryString::from(b64_decode(str_of(v)?)?)),
        VariantType::SharedString => Variant::SharedString(SharedString::new(b64_decode(str_of(v)?)?)),
        VariantType::ContentId => Variant::ContentId(ContentId::from(str_of(v)?)),
        VariantType::Content => {
            let s = str_of(v)?;
            Variant::Content(if s.is_empty() { Content::none() } else { Content::from_uri(s) })
        }
        VariantType::Vector2 => {
            let a = nums(v, 2)?;
            Variant::Vector2(Vector2::new(a[0] as f32, a[1] as f32))
        }
        VariantType::Vector2int16 => {
            let a = nums(v, 2)?;
            Variant::Vector2int16(Vector2int16::new(a[0] as i16, a[1] as i16))
        }
        VariantType::Vector3 => Variant::Vector3(v3(&nums(v, 3)?)),
        VariantType::Vector3int16 => {
            let a = nums(v, 3)?;
            Variant::Vector3int16(Vector3int16::new(a[0] as i16, a[1] as i16, a[2] as i16))
        }
        VariantType::CFrame => Variant::CFrame(cframe_from(&nums(v, 12)?)),
        VariantType::OptionalCFrame => {
            if v.is_null() {
                Variant::OptionalCFrame(None)
            } else {
                Variant::OptionalCFrame(Some(cframe_from(&nums(v, 12)?)))
            }
        }
        VariantType::Color3 => {
            let a = nums(v, 3)?;
            Variant::Color3(Color3::new(a[0] as f32, a[1] as f32, a[2] as f32))
        }
        VariantType::Color3uint8 => {
            let a = nums(v, 3)?;
            Variant::Color3uint8(Color3uint8::new(a[0] as u8, a[1] as u8, a[2] as u8))
        }
        VariantType::BrickColor => {
            let n = num(v)? as u16;
            Variant::BrickColor(BrickColor::from_number(n).ok_or_else(|| anyhow!("unknown BrickColor number {n}"))?)
        }
        VariantType::UDim => {
            let a = nums(v, 2)?;
            Variant::UDim(UDim::new(a[0] as f32, a[1] as i32))
        }
        VariantType::UDim2 => {
            let a = nums(v, 4)?;
            Variant::UDim2(UDim2::new(
                UDim::new(a[0] as f32, a[1] as i32),
                UDim::new(a[2] as f32, a[3] as i32),
            ))
        }
        VariantType::Rect => {
            let a = nums(v, 4)?;
            Variant::Rect(Rect::new(
                Vector2::new(a[0] as f32, a[1] as f32),
                Vector2::new(a[2] as f32, a[3] as f32),
            ))
        }
        VariantType::Ray => {
            let a = nums(v, 6)?;
            Variant::Ray(Ray::new(v3(&a[0..3]), v3(&a[3..6])))
        }
        VariantType::NumberRange => {
            let a = nums(v, 2)?;
            Variant::NumberRange(NumberRange::new(a[0] as f32, a[1] as f32))
        }
        VariantType::NumberSequence => {
            let arr = v.as_array().ok_or_else(|| anyhow!("expected keypoint array"))?;
            let mut keypoints = Vec::with_capacity(arr.len());
            for k in arr {
                let a = nums(k, 3)?;
                keypoints.push(NumberSequenceKeypoint::new(a[0] as f32, a[1] as f32, a[2] as f32));
            }
            Variant::NumberSequence(NumberSequence { keypoints })
        }
        VariantType::ColorSequence => {
            let arr = v.as_array().ok_or_else(|| anyhow!("expected keypoint array"))?;
            let mut keypoints = Vec::with_capacity(arr.len());
            for k in arr {
                let a = nums(k, 4)?;
                keypoints.push(ColorSequenceKeypoint::new(
                    a[0] as f32,
                    Color3::new(a[1] as f32, a[2] as f32, a[3] as f32),
                ));
            }
            Variant::ColorSequence(ColorSequence { keypoints })
        }
        VariantType::PhysicalProperties => {
            if v.is_null() {
                Variant::PhysicalProperties(PhysicalProperties::Default)
            } else {
                // [density, friction, elasticity, frictionWeight, elasticityWeight, (acousticAbsorption)]
                let arr = v.as_array().ok_or_else(|| anyhow!("expected array"))?;
                let a: Vec<f64> = arr.iter().map(num).collect::<Result<_>>()?;
                if a.len() < 5 {
                    bail!("PhysicalProperties needs 5 numbers or null");
                }
                let acoustic = a.get(5).copied().unwrap_or(1.0);
                Variant::PhysicalProperties(PhysicalProperties::Custom(CustomPhysicalProperties::new(
                    a[0] as f32, a[1] as f32, a[2] as f32, a[3] as f32, a[4] as f32, acoustic as f32,
                )))
            }
        }
        VariantType::Enum => Variant::Enum(Enum::from_u32(num(v)? as u32)),
        VariantType::Faces => {
            let bits = num(v)? as u8;
            Variant::Faces(Faces::from_bits(bits).ok_or_else(|| anyhow!("bad Faces bits"))?)
        }
        VariantType::Axes => {
            let bits = num(v)? as u8;
            Variant::Axes(Axes::from_bits(bits).ok_or_else(|| anyhow!("bad Axes bits"))?)
        }
        VariantType::Font => {
            let o = v.as_object().ok_or_else(|| anyhow!("expected font object"))?;
            let family = o.get("family").and_then(Value::as_str).ok_or_else(|| anyhow!("Font.family missing"))?;
            let weight = FontWeight::from_u16(o.get("weight").and_then(Value::as_u64).unwrap_or(400) as u16)
                .unwrap_or_default();
            let style = FontStyle::from_u8(o.get("style").and_then(Value::as_u64).unwrap_or(0) as u8)
                .unwrap_or_default();
            Variant::Font(Font::new(family, weight, style))
        }
        VariantType::Ref => match v {
            Value::Null => Variant::Ref(Ref::none()),
            Value::String(id) => Variant::Ref(refs.get(id).copied().unwrap_or_else(Ref::none)),
            _ => bail!("Ref must be an id string or null"),
        },
        VariantType::Tags => {
            let arr = v.as_array().ok_or_else(|| anyhow!("expected string array"))?;
            let mut tags = Tags::new();
            for t in arr {
                tags.push(str_of(t)?);
            }
            Variant::Tags(tags)
        }
        VariantType::Attributes => {
            let o = v.as_object().ok_or_else(|| anyhow!("expected attribute object"))?;
            let mut attrs = Attributes::new();
            for (name, w) in o {
                let w: Wire = serde_json::from_value(w.clone()).context("bad attribute value")?;
                let aty = parse_type(&w.t).ok_or_else(|| anyhow!("unknown attribute type {}", w.t))?;
                if matches!(aty, VariantType::Attributes | VariantType::Ref | VariantType::Tags) {
                    bail!("attribute {name}: type {} is not allowed", w.t);
                }
                attrs.insert(name.clone(), to_variant(aty, &w.v, refs).with_context(|| format!("attribute {name}"))?);
            }
            Variant::Attributes(attrs)
        }
        other => bail!("unsupported type {other:?}"),
    })
}

// ---------------------------------------------------------------- Variant -> JSON

/// Returns `None` for types the wire format does not carry.
pub fn from_variant(var: &Variant, ids: &HashMap<Ref, String>) -> Option<Wire> {
    let ty = var.ty();
    let name = type_name(ty)?;
    let v: Value = match var {
        Variant::Bool(b) => json!(b),
        Variant::Int32(n) => json!(n),
        Variant::Int64(n) => json!(n),
        Variant::Float32(n) => finite(*n as f64),
        Variant::Float64(n) => finite(*n),
        Variant::String(s) => json!(s),
        Variant::BinaryString(b) => json!(b64_encode(b.as_ref())),
        Variant::SharedString(s) => json!(b64_encode(s.data())),
        Variant::ContentId(c) => json!(c.as_str()),
        Variant::Content(c) => match c.value() {
            ContentType::Uri(u) => json!(u),
            _ => json!(""),
        },
        Variant::Vector2(a) => json!([a.x, a.y]),
        Variant::Vector2int16(a) => json!([a.x, a.y]),
        Variant::Vector3(a) => json!([a.x, a.y, a.z]),
        Variant::Vector3int16(a) => json!([a.x, a.y, a.z]),
        Variant::CFrame(cf) => cframe_to(cf),
        Variant::OptionalCFrame(cf) => cf.as_ref().map(cframe_to).unwrap_or(Value::Null),
        Variant::Color3(c) => json!([c.r, c.g, c.b]),
        Variant::Color3uint8(c) => json!([c.r, c.g, c.b]),
        Variant::BrickColor(b) => json!(*b as u16),
        Variant::UDim(u) => json!([u.scale, u.offset]),
        Variant::UDim2(u) => json!([u.x.scale, u.x.offset, u.y.scale, u.y.offset]),
        Variant::Rect(r) => json!([r.min.x, r.min.y, r.max.x, r.max.y]),
        Variant::Ray(r) => json!([r.origin.x, r.origin.y, r.origin.z, r.direction.x, r.direction.y, r.direction.z]),
        Variant::NumberRange(r) => json!([r.min, r.max]),
        Variant::NumberSequence(s) => Value::Array(s.keypoints.iter().map(|k| json!([k.time, k.value, k.envelope])).collect()),
        Variant::ColorSequence(s) => Value::Array(s.keypoints.iter().map(|k| json!([k.time, k.color.r, k.color.g, k.color.b])).collect()),
        Variant::PhysicalProperties(p) => match p {
            PhysicalProperties::Default => Value::Null,
            PhysicalProperties::Custom(c) => json!([
                c.density(), c.friction(), c.elasticity(), c.friction_weight(), c.elasticity_weight(), c.acoustic_absorption()
            ]),
        },
        Variant::Enum(e) => json!(e.to_u32()),
        Variant::Faces(f) => json!(f.bits()),
        Variant::Axes(a) => json!(a.bits()),
        Variant::Font(f) => json!({ "family": f.family, "weight": f.weight.as_u16(), "style": f.style.as_u8() }),
        Variant::Ref(r) => ids.get(r).map(|s| json!(s)).unwrap_or(Value::Null),
        Variant::Tags(t) => Value::Array(t.iter().map(|s| json!(s)).collect()),
        Variant::Attributes(a) => {
            let mut m = Map::new();
            for (k, val) in a.iter() {
                // rbx_types reads attribute strings back as BinaryString; turn valid UTF-8 into String.
                let w = match val {
                    Variant::BinaryString(b) => match std::str::from_utf8(b.as_ref()) {
                        Ok(s) => Some(Wire { t: "String".to_owned(), v: json!(s), e: None }),
                        Err(_) => from_variant(val, ids),
                    },
                    _ => from_variant(val, ids),
                };
                if let Some(w) = w {
                    m.insert(k.clone(), json!({ "t": w.t, "v": w.v }));
                }
            }
            Value::Object(m)
        }
        _ => return None,
    };
    Some(Wire { t: name.to_owned(), v, e: None })
}

fn finite(n: f64) -> Value {
    if n.is_finite() { json!(n) } else { json!(0) }
}
