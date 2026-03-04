use std::collections::BTreeMap;
use std::sync::Arc;

use trustfall::{FieldValue, TransparentValue};

pub fn transparent_to_field(tv: &TransparentValue) -> FieldValue {
    match tv {
        TransparentValue::String(s) => FieldValue::from(&**s),
        TransparentValue::Float64(f) => FieldValue::Float64(*f),
        TransparentValue::Int64(i) => FieldValue::Int64(*i),
        TransparentValue::Uint64(u) => FieldValue::Uint64(*u),
        TransparentValue::Boolean(b) => FieldValue::Boolean(*b),
        TransparentValue::Null => FieldValue::Null,
        TransparentValue::List(l) => {
            let items: Vec<FieldValue> = l.iter().map(transparent_to_field).collect();
            FieldValue::List(items.into())
        }
        _ => FieldValue::Null,
    }
}

pub fn transparent_to_json(tv: &TransparentValue) -> serde_json::Value {
    match tv {
        TransparentValue::String(s) => serde_json::Value::String(s.to_string()),
        TransparentValue::Float64(f) => serde_json::json!(f),
        TransparentValue::Int64(i) => serde_json::json!(i),
        TransparentValue::Uint64(u) => serde_json::json!(u),
        TransparentValue::Boolean(b) => serde_json::Value::Bool(*b),
        TransparentValue::Null => serde_json::Value::Null,
        TransparentValue::List(l) => {
            let items: Vec<serde_json::Value> = l.iter().map(transparent_to_json).collect();
            serde_json::Value::Array(items)
        }
        _ => serde_json::Value::Null,
    }
}

pub fn field_value_to_json(v: &FieldValue) -> serde_json::Value {
    match v {
        FieldValue::String(s) => serde_json::Value::String(s.to_string()),
        FieldValue::Int64(i) => serde_json::json!(i),
        FieldValue::Uint64(u) => serde_json::json!(u),
        FieldValue::Float64(f) => serde_json::json!(f),
        FieldValue::Boolean(b) => serde_json::Value::Bool(*b),
        FieldValue::Null => serde_json::Value::Null,
        FieldValue::List(l) => {
            let items: Vec<serde_json::Value> = l.iter().map(field_value_to_json).collect();
            serde_json::Value::Array(items)
        }
        other => serde_json::json!(format!("{other:?}")),
    }
}

pub fn row_to_json(row: &BTreeMap<Arc<str>, FieldValue>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = row
        .iter()
        .map(|(k, v)| (k.to_string(), field_value_to_json(v)))
        .collect();
    serde_json::Value::Object(map)
}

pub fn json_to_field_value(v: &serde_json::Value) -> FieldValue {
    match v {
        serde_json::Value::String(s) => FieldValue::from(s.as_str()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                FieldValue::Int64(i)
            } else if let Some(u) = n.as_u64() {
                FieldValue::Uint64(u)
            } else if let Some(f) = n.as_f64() {
                FieldValue::Float64(f)
            } else {
                FieldValue::Null
            }
        }
        serde_json::Value::Bool(b) => FieldValue::Boolean(*b),
        serde_json::Value::Null => FieldValue::Null,
        serde_json::Value::Array(arr) => {
            let items: Vec<FieldValue> = arr.iter().map(json_to_field_value).collect();
            FieldValue::List(items.into())
        }
        serde_json::Value::Object(_) => FieldValue::Null,
    }
}
