//! Database value types
//!
//! This module defines the types that can be stored and retrieved from databases.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database value that can hold different types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatabaseValue {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// 32-bit integer
    Int(i32),
    /// 64-bit integer
    Long(i64),
    /// 32-bit floating point
    Float(f32),
    /// 64-bit floating point
    Double(f64),
    /// String value
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Timestamp (Unix timestamp in microseconds)
    Timestamp(i64),
}

impl DatabaseValue {
    /// Get the value as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DatabaseValue::Bool(v) => Some(*v),
            DatabaseValue::Int(v) => Some(*v != 0),
            DatabaseValue::Long(v) => Some(*v != 0),
            DatabaseValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Get the value as an i32
    pub fn as_int(&self) -> Option<i32> {
        match self {
            DatabaseValue::Int(v) => Some(*v),
            DatabaseValue::Long(v) => i32::try_from(*v).ok(),
            DatabaseValue::Float(v) => Some(*v as i32),
            DatabaseValue::Double(v) => Some(*v as i32),
            DatabaseValue::String(s) => s.parse().ok(),
            DatabaseValue::Bool(v) => Some(*v as i32),
            _ => None,
        }
    }

    /// Get the value as an i64
    pub fn as_long(&self) -> Option<i64> {
        match self {
            DatabaseValue::Long(v) => Some(*v),
            DatabaseValue::Int(v) => Some(*v as i64),
            DatabaseValue::Float(v) => Some(*v as i64),
            DatabaseValue::Double(v) => Some(*v as i64),
            DatabaseValue::String(s) => s.parse().ok(),
            DatabaseValue::Bool(v) => Some(*v as i64),
            DatabaseValue::Timestamp(v) => Some(*v),
            _ => None,
        }
    }

    /// Get the value as an f32
    pub fn as_float(&self) -> Option<f32> {
        match self {
            DatabaseValue::Float(v) => Some(*v),
            DatabaseValue::Double(v) => Some(*v as f32),
            DatabaseValue::Int(v) => Some(*v as f32),
            DatabaseValue::Long(v) => Some(*v as f32),
            DatabaseValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Get the value as an f64
    pub fn as_double(&self) -> Option<f64> {
        match self {
            DatabaseValue::Double(v) => Some(*v),
            DatabaseValue::Float(v) => Some(*v as f64),
            DatabaseValue::Int(v) => Some(*v as f64),
            DatabaseValue::Long(v) => Some(*v as f64),
            DatabaseValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Get the value as a string (zero-copy for String values)
    ///
    /// Returns a string reference without cloning for String values.
    /// For other types, use `as_string()` which performs conversion.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DatabaseValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get the value as a string (with conversion)
    pub fn as_string(&self) -> String {
        match self {
            DatabaseValue::Null => "null".to_string(),
            DatabaseValue::Bool(v) => v.to_string(),
            DatabaseValue::Int(v) => v.to_string(),
            DatabaseValue::Long(v) => v.to_string(),
            DatabaseValue::Float(v) => v.to_string(),
            DatabaseValue::Double(v) => v.to_string(),
            DatabaseValue::String(s) => s.clone(),
            DatabaseValue::Bytes(b) => format!("<{} bytes>", b.len()),
            DatabaseValue::Timestamp(v) => v.to_string(),
        }
    }

    /// Get the value as bytes (zero-copy)
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            DatabaseValue::Bytes(b) => Some(b),
            DatabaseValue::String(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, DatabaseValue::Null)
    }

    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            DatabaseValue::Null => "null",
            DatabaseValue::Bool(_) => "bool",
            DatabaseValue::Int(_) => "int",
            DatabaseValue::Long(_) => "long",
            DatabaseValue::Float(_) => "float",
            DatabaseValue::Double(_) => "double",
            DatabaseValue::String(_) => "string",
            DatabaseValue::Bytes(_) => "bytes",
            DatabaseValue::Timestamp(_) => "timestamp",
        }
    }
}

impl From<bool> for DatabaseValue {
    fn from(v: bool) -> Self {
        DatabaseValue::Bool(v)
    }
}

impl From<i32> for DatabaseValue {
    fn from(v: i32) -> Self {
        DatabaseValue::Int(v)
    }
}

impl From<i64> for DatabaseValue {
    fn from(v: i64) -> Self {
        DatabaseValue::Long(v)
    }
}

impl From<f32> for DatabaseValue {
    fn from(v: f32) -> Self {
        DatabaseValue::Float(v)
    }
}

impl From<f64> for DatabaseValue {
    fn from(v: f64) -> Self {
        DatabaseValue::Double(v)
    }
}

impl From<String> for DatabaseValue {
    fn from(v: String) -> Self {
        DatabaseValue::String(v)
    }
}

impl From<&str> for DatabaseValue {
    fn from(v: &str) -> Self {
        DatabaseValue::String(v.to_string())
    }
}

impl From<Vec<u8>> for DatabaseValue {
    fn from(v: Vec<u8>) -> Self {
        DatabaseValue::Bytes(v)
    }
}

impl<T: Into<DatabaseValue>> From<Option<T>> for DatabaseValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => DatabaseValue::Null,
        }
    }
}

/// A row of database results (column name -> value mapping)
pub type DatabaseRow = HashMap<String, DatabaseValue>;

/// Multiple rows returned from a query
pub type DatabaseResult = Vec<DatabaseRow>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        let val = DatabaseValue::Int(42);
        assert_eq!(val.as_int(), Some(42));
        assert_eq!(val.as_long(), Some(42));
        assert_eq!(val.as_string(), "42");

        let val = DatabaseValue::String("123".to_string());
        assert_eq!(val.as_int(), Some(123));
        assert_eq!(val.as_long(), Some(123));

        let val = DatabaseValue::Bool(true);
        assert_eq!(val.as_bool(), Some(true));
        assert_eq!(val.as_int(), Some(1));
    }

    #[test]
    fn test_value_from_types() {
        let val: DatabaseValue = 42.into();
        assert_eq!(val, DatabaseValue::Int(42));

        let val: DatabaseValue = "hello".into();
        assert_eq!(val, DatabaseValue::String("hello".to_string()));

        let val: DatabaseValue = true.into();
        assert_eq!(val, DatabaseValue::Bool(true));

        let val: DatabaseValue = Some(42).into();
        assert_eq!(val, DatabaseValue::Int(42));

        let val: DatabaseValue = Option::<i32>::None.into();
        assert_eq!(val, DatabaseValue::Null);
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(DatabaseValue::Null.type_name(), "null");
        assert_eq!(DatabaseValue::Bool(true).type_name(), "bool");
        assert_eq!(DatabaseValue::Int(42).type_name(), "int");
        assert_eq!(DatabaseValue::Long(42).type_name(), "long");
        assert_eq!(
            DatabaseValue::String("test".to_string()).type_name(),
            "string"
        );
    }
}
