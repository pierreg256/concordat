//! CrdtValue — recursive value type (Object, Array, Scalar).
//!
//! Ties together OR-Map, RGA, and MV-Register into a single recursive JSON model.

use serde::{Deserialize, Serialize};

use crate::ormap::OrMap;
use crate::register::MvRegister;
use crate::rga::Rga;
use crate::vv::{Dot, VersionVector};

/// A CRDT value: the recursive type that models JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtValue {
    /// A scalar JSON value (string, number, bool, null) stored in a MV-Register.
    Scalar(MvRegister<serde_json::Value>),
    /// A JSON object stored in an OR-Map.
    Object(OrMap<String, CrdtValue>),
    /// A JSON array stored in an RGA.
    Array(Rga<CrdtValue>),
}

impl CrdtValue {
    /// Create a scalar value.
    pub fn scalar(value: serde_json::Value) -> Self {
        let mut reg = MvRegister::new();
        // We don't set here — the caller (CrdtDoc) will call set with proper dot/vv
        reg.set(
            value,
            Dot {
                replica: String::new(),
                counter: 0,
            },
            &VersionVector::new(),
        );
        CrdtValue::Scalar(reg)
    }

    /// Create an empty object.
    pub fn object() -> Self {
        CrdtValue::Object(OrMap::new())
    }

    /// Create an empty array.
    pub fn array() -> Self {
        CrdtValue::Array(Rga::new())
    }

    /// Merge another CrdtValue into this one.
    ///
    /// If types match, merge recursively.
    /// If types mismatch (concurrent type change), the value with higher
    /// type priority wins: Object > Array > Scalar.
    pub fn merge(&mut self, other: &CrdtValue) {
        match (self, other) {
            (CrdtValue::Scalar(a), CrdtValue::Scalar(b)) => a.merge(b),
            (CrdtValue::Object(a), CrdtValue::Object(b)) => a.merge(b),
            (CrdtValue::Array(a), CrdtValue::Array(b)) => a.merge(b),
            // Type mismatch: higher priority wins
            (s, o) => {
                if type_priority(o) > type_priority(s) {
                    *s = o.clone();
                }
                // else: self has higher or equal priority, keep self
            }
        }
    }

    /// Materialize this CRDT value into a plain serde_json::Value.
    pub fn materialize(&self) -> serde_json::Value {
        match self {
            CrdtValue::Scalar(reg) => reg.value().cloned().unwrap_or(serde_json::Value::Null),
            CrdtValue::Object(map) => {
                let mut obj = serde_json::Map::new();
                for key in map.keys() {
                    if let Some(value) = map.get(key) {
                        obj.insert(key.clone(), value.materialize());
                    }
                }
                serde_json::Value::Object(obj)
            }
            CrdtValue::Array(rga) => {
                let arr: Vec<serde_json::Value> = rga.iter().map(|v| v.materialize()).collect();
                serde_json::Value::Array(arr)
            }
        }
    }
}

impl PartialEq for CrdtValue {
    fn eq(&self, other: &Self) -> bool {
        self.materialize() == other.materialize()
    }
}

impl Eq for CrdtValue {}

/// Type priority for conflict resolution on type mismatch.
fn type_priority(v: &CrdtValue) -> u8 {
    match v {
        CrdtValue::Scalar(_) => 0,
        CrdtValue::Array(_) => 1,
        CrdtValue::Object(_) => 2,
    }
}
