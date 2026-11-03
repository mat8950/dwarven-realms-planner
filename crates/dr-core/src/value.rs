//! Runtime values of the blueprint VM. Structs and containers have value semantics
//! (cloned on copy), like in Unreal.

use std::fmt;

use crate::bytecode::Str;

pub type ObjId = usize;

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    None,
    Bool(bool),
    Byte(u8),
    Int(i64),
    Float(f64),
    Name(Str),
    Str(Str),
    Struct(Box<StructVal>),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Object(ObjId),
    /// Asset or class reference that is not a live object (data tables, libraries).
    Asset(Str),
}

#[derive(Clone, Debug)]
pub struct StructVal {
    pub ty: Str,
    pub fields: Vec<(Str, Value)>,
}

impl StructVal {
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| &**k == field).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, field: &str) -> &mut Value {
        if let Some(i) = self.fields.iter().position(|(k, _)| &**k == field) {
            return &mut self.fields[i].1;
        }
        self.fields.push((field.into(), Value::None));
        &mut self.fields.last_mut().unwrap().1
    }
}

impl Value {
    pub fn f64(&self) -> f64 {
        match self {
            Value::Float(f) => *f,
            Value::Int(i) => *i as f64,
            Value::Byte(b) => *b as f64,
            Value::Bool(b) => *b as i64 as f64,
            _ => 0.0,
        }
    }

    pub fn i64(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            Value::Byte(b) => *b as i64,
            Value::Float(f) => *f as i64,
            Value::Bool(b) => *b as i64,
            _ => 0,
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Byte(b) => *b != 0,
            Value::Object(_) => true,
            _ => false,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Value::Name(s) | Value::Str(s) | Value::Asset(s) => s,
            Value::Struct(s) if s.ty.as_ref() == "GameplayTag" => s.get("TagName").map(|v| v.name()).unwrap_or("None"),
            _ => "None",
        }
    }

    pub fn tag(name: &str) -> Value {
        Value::Struct(Box::new(StructVal { ty: "GameplayTag".into(), fields: vec![("TagName".into(), Value::Name(name.into()))] }))
    }

    pub fn is_none_object(&self) -> bool {
        matches!(self, Value::None)
    }

    /// Equality used by map keys and `==` on names, tags, bytes.
    pub fn key_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Struct(a), Value::Struct(b)) => a.ty == b.ty && a.fields.len() == b.fields.len() && a.fields.iter().zip(&b.fields).all(|(x, y)| x.0 == y.0 && x.1.key_eq(&y.1)),
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Name(_) | Value::Str(_) | Value::Asset(_), _) | (_, Value::Name(_) | Value::Str(_) | Value::Asset(_)) => {
                self.name().eq_ignore_ascii_case(other.name())
            }
            (Value::None, Value::None) => true,
            (Value::None, _) | (_, Value::None) => false,
            _ => self.f64() == other.f64(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Byte(b) => write!(f, "{b}u8"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Name(s) => write!(f, "{s}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Asset(s) => write!(f, "<{s}>"),
            Value::Object(o) => write!(f, "obj#{o}"),
            Value::Array(a) => write!(f, "[{} items]", a.len()),
            Value::Map(m) => write!(f, "{{{} entries}}", m.len()),
            Value::Struct(s) => {
                write!(f, "{}{{", s.ty)?;
                for (i, (k, v)) in s.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {v}", short_field(k))?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// `Value_5_43BD33CA455ADE70795B28BCBCDB22CD` -> `Value`.
pub fn short_field(name: &str) -> &str {
    let b = name.as_bytes();
    if b.len() > 34 && b[b.len() - 33] == b'_' && b[b.len() - 32..].iter().all(|c| c.is_ascii_hexdigit()) {
        let head = &name[..name.len() - 33];
        if let Some(p) = head.rfind('_') {
            if head[p + 1..].bytes().all(|c| c.is_ascii_digit()) {
                return &head[..p];
            }
        }
    }
    name
}
