//! The SmileBASIC value model.
//!
//! Ported from the semantics of `osb/SMILEBASIC/type.d`:
//! - **Integer** is 32-bit (`i32`), **Double** is 64-bit IEEE (`f64`) — SmileBASIC's
//!   "real number" type.
//! - **String** (`$`) is a *mutable UTF-16* sequence.
//! - **Array** is 1–4 dimensional, row-major, and is held by reference.
//!
//! Int↔Double coercion happens on assignment and inside operators, exactly as SB3.
//! Arrays and references land in milestone M1; the [`ValueType`] tags for them are
//! defined now so the rest of the core can refer to them.

use crate::error::{ErrNum, SbError, SbResult};

/// A SmileBASIC string: a mutable UTF-16 code-unit buffer.
pub type SbString = Vec<u16>;

/// The full set of runtime value tags (mirrors `ValueType` in `type.d`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Void,
    Integer,
    Double,
    String,
    // --- M1: arrays, references, internal address/data/function tags ---
    Array,
    IntegerArray,
    DoubleArray,
    StringArray,
    InternalAddress,
    InternalSlotAddress,
    Reference,
    IntegerReference,
    DoubleReference,
    StringReference,
    StringArrayReference,
    Data,
    Function,
}

/// A runtime value. (Scalars + strings implemented in M0; arrays/refs follow in M1.)
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Double(f64),
    Str(SbString),
}

impl Value {
    /// Build a string value from a Rust `&str` (UTF-8 -> UTF-16).
    pub fn string(s: &str) -> Self {
        Value::Str(s.encode_utf16().collect())
    }

    /// The runtime type tag.
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Int(_) => ValueType::Integer,
            Value::Double(_) => ValueType::Double,
            Value::Str(_) => ValueType::String,
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Double(_))
    }

    /// SmileBASIC truthiness: non-zero number, or non-empty string.
    /// (See `type.d:boolValue` — strings became truthy in 3.1.)
    pub fn is_true(&self) -> bool {
        match self {
            Value::Int(i) => *i != 0,
            Value::Double(d) => *d != 0.0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    /// Coerce to `i32`. Double truncates toward zero (verify rounding vs hardware);
    /// String is a `Type mismatch`.
    pub fn to_int(&self) -> SbResult<i32> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::Double(d) => Ok(*d as i32),
            Value::Str(_) => Err(SbError::new(ErrNum::TypeMismatch)),
        }
    }

    /// Coerce to `f64`. String is a `Type mismatch`.
    pub fn to_double(&self) -> SbResult<f64> {
        match self {
            Value::Int(i) => Ok(*i as f64),
            Value::Double(d) => Ok(*d),
            Value::Str(_) => Err(SbError::new(ErrNum::TypeMismatch)),
        }
    }

    /// Borrow as a UTF-16 string, or `Type mismatch` for numbers.
    pub fn as_str(&self) -> SbResult<&SbString> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(SbError::new(ErrNum::TypeMismatch)),
        }
    }

    /// Lossy UTF-16 -> Rust `String` (for tests / debug / PRINT scaffolding).
    pub fn to_rust_string_lossy(&self) -> String {
        match self {
            Value::Str(s) => String::from_utf16_lossy(s),
            Value::Int(i) => i.to_string(),
            // NOTE: this is NOT SB3's STR$ formatting — that algorithm is reverse-
            // engineered from the disassembly in M1. This is debug-only.
            Value::Double(d) => d.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthiness_matches_sb3() {
        assert!(!Value::Int(0).is_true());
        assert!(Value::Int(1).is_true());
        assert!(!Value::Double(0.0).is_true());
        assert!(!Value::string("").is_true());
        assert!(Value::string("A").is_true());
    }

    #[test]
    fn string_coercion_is_type_mismatch() {
        assert_eq!(
            Value::string("x").to_int().unwrap_err().num,
            ErrNum::TypeMismatch
        );
    }

    #[test]
    fn utf16_roundtrip() {
        assert_eq!(Value::string("HELLO").to_rust_string_lossy(), "HELLO");
    }
}
