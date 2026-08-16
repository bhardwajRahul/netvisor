//! Tolerant scalars for vendor JSON that is inconsistent about types.
//!
//! Management APIs quote their numbers in some firmware versions and not others, and send
//! booleans as `true`, `"true"` or `1` depending on the device class. serde's default would abort
//! the whole response parse on the first mismatch — leaving the topology empty, which is the
//! symptom these integrations exist to fix. Every numeric and boolean field in a vendor wire
//! struct goes through these instead.
//!
//! Lifted out of the UniFi types when Instant On needed the same guarantee against a
//! reverse-engineered cloud API. The reasoning is not vendor-specific, so neither is the code.

use serde::{Deserialize, Deserializer};

/// An integer that may arrive as a JSON number *or* a quoted string.
///
/// Not defensive over-engineering: the unpoller/unifi Go library wraps every numeric in its own
/// equivalent for exactly this reason.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlexInt(pub i64);

impl FlexInt {
    pub fn as_i64(self) -> i64 {
        self.0
    }
    pub fn as_i32(self) -> i32 {
        // Port indexes and speeds are far inside i32; clamp rather than wrap on absurd input.
        self.0.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

impl<'de> Deserialize<'de> for FlexInt {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Number(n) => Ok(FlexInt(
                n.as_i64()
                    .or_else(|| n.as_f64().map(|f| f as i64))
                    .unwrap_or(0),
            )),
            serde_json::Value::String(s) => {
                Ok(FlexInt(s.trim().parse::<i64>().unwrap_or_else(|_| {
                    s.trim().parse::<f64>().map(|f| f as i64).unwrap_or(0)
                })))
            }
            serde_json::Value::Bool(b) => Ok(FlexInt(b as i64)),
            _ => Ok(FlexInt(0)),
        }
    }
}

/// A boolean that may arrive as a JSON bool, a quoted string, or 0/1. See [`FlexInt`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlexBool(pub bool);

impl FlexBool {
    pub fn as_bool(self) -> bool {
        self.0
    }
}

impl<'de> Deserialize<'de> for FlexBool {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Bool(b) => Ok(FlexBool(b)),
            serde_json::Value::Number(n) => Ok(FlexBool(n.as_i64().unwrap_or(0) != 0)),
            serde_json::Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(FlexBool(true)),
                _ => Ok(FlexBool(false)),
            },
            _ => Ok(FlexBool(false)),
        }
    }
}
