//! Reads a character `.sav` (Unreal SaveGame) into VM values.

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value as Json;

use crate::value::{StructVal, Value};

/// Root properties of a save, field names without their `_0` array suffix.
pub fn load(path: &Path) -> Result<StructVal> {
    let mut buf = Vec::new();
    BufReader::new(File::open(path).with_context(|| format!("opening {}", path.display()))?).read_to_end(&mut buf)?;
    let save = uesave::Save::read(&mut Cursor::new(buf)).map_err(|e| anyhow::anyhow!("{e}"))?;
    let json = serde_json::to_value(&save)?;
    let props = json["root"]["properties"].as_object().context("save without properties")?;
    Ok(StructVal { ty: "SaveGame".into(), fields: props.iter().map(|(k, v)| (strip_index(k).into(), convert(v))).collect() })
}

/// `Level_11_0D1B..._0` -> `Level_11_0D1B...`.
fn strip_index(k: &str) -> &str {
    match k.rsplit_once('_') {
        Some((head, tail)) if !tail.is_empty() && tail.bytes().all(|c| c.is_ascii_digit()) => head,
        _ => k,
    }
}

fn enum_byte(label: &str) -> Value {
    match crate::gamedata::enum_value(label) {
        Some(n) => Value::Byte(n),
        None => Value::Name(label.into()),
    }
}

fn fields(m: &serde_json::Map<String, Json>, ty: &str) -> Value {
    Value::Struct(Box::new(StructVal { ty: ty.into(), fields: m.iter().map(|(k, v)| (strip_index(k).into(), convert(v))).collect() }))
}

const STRUCT_MARKERS: &[&str] = &["Struct", "Quat", "Vector", "Vector2D", "Rotator", "DateTime", "Timespan", "Guid", "LinearColor", "Color", "IntPoint"];

/// Converts one uesave property value (tagged or not) to a VM value.
fn convert(v: &Json) -> Value {
    let Some(o) = v.as_object() else {
        return scalar(v);
    };
    let Some((kind, inner)) = o.iter().find(|(k, _)| k.as_str() != "tag") else { return Value::None };
    match kind.as_str() {
        "Int" | "Int8" | "Int16" | "Int64" | "UInt16" | "UInt32" | "UInt64" => Value::Int(inner.as_i64().unwrap_or(0)),
        "Float" | "Double" => Value::Float(inner.as_f64().unwrap_or(0.0)),
        "Bool" => Value::Bool(inner.as_bool().unwrap_or(false)),
        "Str" | "Name" => Value::Name(inner.as_str().unwrap_or("").into()),
        "Text" => Value::Str(text(inner).as_str().into()),
        "Byte" => match inner {
            Json::Number(n) => Value::Byte(n.as_u64().unwrap_or(0) as u8),
            Json::Object(m) => enum_byte(m.get("Label").and_then(|x| x.as_str()).unwrap_or("")),
            _ => Value::Byte(0),
        },
        "Enum" => enum_byte(inner.as_str().unwrap_or("")),
        "Object" | "SoftObject" => {
            let s = inner.as_str().unwrap_or("");
            if s.is_empty() { Value::None } else { Value::Asset(s.rsplit('.').next().unwrap_or(s).into()) }
        }
        "Struct" => {
            let ty = o.get("tag").and_then(|t| t["data"]["Struct"]["struct_type"].as_object()).and_then(|m| m.values().next()).and_then(|x| x.as_str()).unwrap_or("");
            let Some(m) = inner.as_object() else { return scalar(inner) };
            // Tagged form wraps the fields again ({"Struct": {...}}); special structs use their type name.
            if m.len() == 1 {
                let (k, x) = m.iter().next().unwrap();
                if STRUCT_MARKERS.contains(&k.as_str()) {
                    return match x {
                        Json::Object(f) => fields(f, if ty.is_empty() { k } else { ty }),
                        other => scalar(other),
                    };
                }
            }
            fields(m, ty)
        }
        "Array" | "Set" => {
            let Some(m) = inner.as_object() else { return Value::Array(Vec::new()) };
            if let Some(s) = m.get("Struct") {
                let items = s["value"].as_array().into_iter().flatten().map(convert).collect();
                return Value::Array(items);
            }
            let base = m.get("Base").and_then(|b| b.as_object()).unwrap_or(m);
            let items = base.iter().next().map(|(k, arr)| arr.as_array().into_iter().flatten().map(|x| convert(&serde_json::json!({ k.clone(): x }))).collect()).unwrap_or_default();
            Value::Array(items)
        }
        "Map" => Value::Map(inner.as_array().into_iter().flatten().map(|e| (convert(&e["key"]), convert(&e["value"]))).collect()),
        _ => match inner {
            Json::Object(m) => fields(m, kind),
            other => scalar(other),
        },
    }
}

fn scalar(v: &Json) -> Value {
    match v {
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) if n.is_f64() => Value::Float(n.as_f64().unwrap_or(0.0)),
        Json::Number(n) => Value::Int(n.as_i64().unwrap_or(0)),
        Json::String(s) => Value::Name(s.as_str().into()),
        Json::Array(a) => Value::Array(a.iter().map(convert).collect()),
        _ => Value::None,
    }
}

fn text(v: &Json) -> String {
    // uesave text: nested history variants; take the first string we find.
    match v {
        Json::String(s) => s.clone(),
        Json::Object(m) => m.values().map(text).find(|s| !s.is_empty()).unwrap_or_default(),
        Json::Array(a) => a.iter().map(text).find(|s| !s.is_empty()).unwrap_or_default(),
        _ => String::new(),
    }
}
