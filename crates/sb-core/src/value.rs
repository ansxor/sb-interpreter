//! Runtime values (M1-T4) — the `Value` the VM pushes, stores and operates on.
//!
//! SmileBASIC has three scalar types — **Integer** (`i32`), **Double** (`f64`) and
//! **String** (mutable UTF-16) — plus arrays of each (1–4D, [`crate::array`]). A
//! variable's *static* type comes from its name suffix (`%`→Int, `#`→Real, `$`→Str,
//! none→dynamic numeric), per `spec/concepts/execution-model.md`. This module owns
//! the value representation and the **coercion rules** the VM applies on assignment
//! and in arithmetic.
//!
//! Strings are stored as `Vec<u16>` ([`SbStr`]) — SmileBASIC source and strings are
//! UTF-16. Arrays are reference types ([`crate::array::ArrayRef`] = `Rc<RefCell<…>>`,
//! single-threaded / wasm-safe). Scalar **references** (for `OUT`/by-ref args and
//! `SWAP`) are modelled by storing each variable in a [`Cell`] = `Rc<RefCell<Value>>`
//! and passing a cloned `Rc` as [`Value::Ref`]; arrays pass by sharing their `Rc`
//! directly (no separate ref variant needed).
//!
//! ## Coercion (hw_verified)
//!
//! Assigning a Double to an Integer target (a `%` variable or integer-array element)
//! **truncates toward zero** — confirmed on real SB 3.6.0 (sb-oracle, see the
//! hw_verified cases in `spec/instructions/var.yaml` + `dim.yaml`): `2.7→2`, `2.5→2`,
//! `4.5→4`, `-2.7→-2`, `-2.5→-2`,
//! `-3.5→-3`. It is **not** rounding and **not** floor. A suffix-less numeric keeps
//! the assigned value's own type (`A=2.7` stays Double; `A=5` stays Integer) — it is
//! not coerced to Integer. Assigning across the numeric/String divide raises **Type
//! mismatch** (errnum 8). This matches osb's `cast(int)` (`VM.d` PopG/PopL), which we
//! confirmed against the oracle rather than inherited.

use crate::array::ArrayRef;
use crate::token::Suffix;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// `Type mismatch` — an inconsistent variable type is specified.
const ERR_TYPE_MISMATCH: u32 = 8;

/// A SmileBASIC string: mutable UTF-16 code units.
pub type SbStr = Vec<u16>;

/// A variable storage cell. Every variable lives in one of these so that `OUT`/
/// by-ref args ([`Value::Ref`]) and `SWAP` can alias and mutate it.
pub type Cell = Rc<RefCell<Value>>;

/// A runtime error carrying a SmileBASIC error number (`spec/reference/errors.yaml`).
/// `value.rs`/`array.rs` raise these directly; the fuller error model (ERRLINE/ERRPRG,
/// halt/CONT) is M1-T13.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    /// The SmileBASIC `ERRNUM`.
    pub errnum: u32,
}

impl RuntimeError {
    pub fn new(errnum: u32) -> Self {
        RuntimeError { errnum }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SmileBASIC error (errnum {})", self.errnum)
    }
}

impl std::error::Error for RuntimeError {}

/// The dynamic type of a [`Value`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// Uninitialised / a `DEF` with no return (`ValueType.Void` in osb).
    Void,
    Int,
    Real,
    Str,
    IntArray,
    RealArray,
    StrArray,
    /// A scalar reference cell.
    Ref,
}

/// A SmileBASIC runtime value.
#[derive(Debug, Clone, Default)]
pub enum Value {
    /// Uninitialised value (e.g. a fresh `DEF` return slot).
    #[default]
    Void,
    /// Integer (`i32`).
    Int(i32),
    /// Double (`f64`).
    Real(f64),
    /// String (mutable UTF-16).
    Str(SbStr),
    /// 1–4D integer array (shared).
    IntArray(ArrayRef<i32>),
    /// 1–4D double array (shared).
    RealArray(ArrayRef<f64>),
    /// 1–4D string array (shared).
    StrArray(ArrayRef<SbStr>),
    /// A reference to a scalar variable cell (`OUT`/by-ref arg, `SWAP`).
    Ref(Cell),
}

impl Value {
    /// The dynamic type tag.
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Void => ValueType::Void,
            Value::Int(_) => ValueType::Int,
            Value::Real(_) => ValueType::Real,
            Value::Str(_) => ValueType::Str,
            Value::IntArray(_) => ValueType::IntArray,
            Value::RealArray(_) => ValueType::RealArray,
            Value::StrArray(_) => ValueType::StrArray,
            Value::Ref(_) => ValueType::Ref,
        }
    }

    /// Build a [`Value::Str`] from a Rust `&str` (UTF-16 encoded).
    pub fn str_from(s: &str) -> Value {
        Value::Str(s.encode_utf16().collect())
    }

    /// Whether this is a numeric scalar (Integer or Double).
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Real(_))
    }

    /// Whether this is any array.
    pub fn is_array(&self) -> bool {
        matches!(
            self,
            Value::IntArray(_) | Value::RealArray(_) | Value::StrArray(_)
        )
    }

    // --- numeric coercion (hw_verified, see module docs) ----------------------

    /// Coerce to `i32`, **truncating toward zero** for Doubles (`2.7→2`, `-2.7→-2`).
    /// Non-numeric values raise Type mismatch (errnum 8).
    pub fn to_int(&self) -> Result<i32, RuntimeError> {
        match self {
            Value::Int(i) => Ok(*i),
            // `f64 as i32` truncates toward zero and saturates on overflow (ARM
            // VCVT-style). Exact huge-magnitude overflow behaviour is oracle-queued.
            Value::Real(d) => Ok(*d as i32),
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        }
    }

    /// Coerce to `f64`. Non-numeric values raise Type mismatch (errnum 8).
    pub fn to_real(&self) -> Result<f64, RuntimeError> {
        match self {
            Value::Int(i) => Ok(*i as f64),
            Value::Real(d) => Ok(*d),
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        }
    }

    /// Borrow the UTF-16 string, or Type mismatch (errnum 8) if not a String.
    pub fn as_str(&self) -> Result<&SbStr, RuntimeError> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        }
    }

    /// Coerce a freshly-computed value to the static type implied by an assignment
    /// **target's suffix**, as the VM does when storing to a variable:
    ///
    /// - `%` → Integer (Double truncates toward zero);
    /// - `#` → Double (Integer widens);
    /// - `$` → String (numeric → Type mismatch);
    /// - none → a numeric keeps its own runtime type (no coercion); a String → Type
    ///   mismatch (a suffix-less name is numeric).
    ///
    /// Arrays and references are returned unchanged.
    pub fn coerce_to_suffix(self, suffix: Suffix) -> Result<Value, RuntimeError> {
        match suffix {
            Suffix::Int => Ok(Value::Int(self.to_int()?)),
            Suffix::Real => Ok(Value::Real(self.to_real()?)),
            Suffix::Str => match self {
                Value::Str(_) => Ok(self),
                _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
            },
            Suffix::None => match self {
                Value::Int(_) | Value::Real(_) | Value::Void => Ok(self),
                // arrays/refs flow through a suffix-less array/ref name unchanged.
                Value::IntArray(_) | Value::RealArray(_) | Value::StrArray(_) | Value::Ref(_) => {
                    Ok(self)
                }
                Value::Str(_) => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
            },
        }
    }

    /// The zero value for a variable declared with `suffix` (`dim.yaml`: numeric → 0,
    /// string → ""). A suffix-less numeric defaults to Integer `0`.
    pub fn default_for_suffix(suffix: Suffix) -> Value {
        match suffix {
            Suffix::Int | Suffix::None => Value::Int(0),
            Suffix::Real => Value::Real(0.0),
            Suffix::Str => Value::Str(SbStr::new()),
        }
    }

    // --- references (OUT / by-ref / SWAP) -------------------------------------

    /// Wrap a value in a fresh storage [`Cell`].
    pub fn cell(v: Value) -> Cell {
        Rc::new(RefCell::new(v))
    }

    /// Make a [`Value::Ref`] aliasing `cell`.
    pub fn ref_to(cell: &Cell) -> Value {
        Value::Ref(Rc::clone(cell))
    }

    /// Read through a [`Value::Ref`] (one level), returning a clone of the pointed
    /// value; a non-reference returns a clone of itself.
    pub fn deref(&self) -> Value {
        match self {
            Value::Ref(cell) => cell.borrow().clone(),
            other => other.clone(),
        }
    }

    /// Write `v` through a [`Value::Ref`]. Errors (errnum 8) if `self` is not a Ref.
    pub fn assign_through(&self, v: Value) -> Result<(), RuntimeError> {
        match self {
            Value::Ref(cell) => {
                *cell.borrow_mut() = v;
                Ok(())
            }
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        }
    }
}

/// Exchange the contents of two storage cells (`SWAP`). Aliased cells (the same `Rc`)
/// are a no-op.
pub fn swap_cells(a: &Cell, b: &Cell) {
    if Rc::ptr_eq(a, b) {
        return;
    }
    std::mem::swap(&mut *a.borrow_mut(), &mut *b.borrow_mut());
}

/// Value equality used by tests / `==` lowering helpers. Numbers compare across
/// Int/Real by f64 value; strings by code units; arrays by shared identity *or*
/// element-wise equality; everything else by structure. (The VM's `==` operator is
/// M1-T6; this is a structural helper.)
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Void, Value::Void) => true,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::IntArray(a), Value::IntArray(b)) => {
                Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow()
            }
            (Value::RealArray(a), Value::RealArray(b)) => {
                Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow()
            }
            (Value::StrArray(a), Value::StrArray(b)) => {
                Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow()
            }
            (Value::Ref(a), Value::Ref(b)) => Rc::ptr_eq(a, b),
            _ => match (self.to_real(), other.to_real()) {
                (Ok(x), Ok(y)) => x == y,
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_to_int_truncates_toward_zero() {
        // hw_verified (sb-oracle): A%=<x> truncates toward zero.
        assert_eq!(Value::Real(2.7).to_int().unwrap(), 2);
        assert_eq!(Value::Real(2.5).to_int().unwrap(), 2);
        assert_eq!(Value::Real(3.5).to_int().unwrap(), 3);
        assert_eq!(Value::Real(4.5).to_int().unwrap(), 4);
        assert_eq!(Value::Real(-2.7).to_int().unwrap(), -2);
        assert_eq!(Value::Real(-2.5).to_int().unwrap(), -2);
        assert_eq!(Value::Real(-3.5).to_int().unwrap(), -3);
        assert_eq!(Value::Int(5).to_int().unwrap(), 5);
    }

    #[test]
    fn int_widens_to_real() {
        assert_eq!(Value::Int(2).to_real().unwrap(), 2.0);
        assert_eq!(Value::Real(2.5).to_real().unwrap(), 2.5);
    }

    #[test]
    fn non_numeric_to_number_is_type_mismatch() {
        assert_eq!(Value::str_from("x").to_int().unwrap_err().errnum, 8);
        assert_eq!(Value::str_from("x").to_real().unwrap_err().errnum, 8);
    }

    #[test]
    fn coerce_to_suffix_int_truncates() {
        // A%=2.7 -> Int(2)  (hw_verified iarr_coerce/ipos_27).
        assert_eq!(
            Value::Real(2.7).coerce_to_suffix(Suffix::Int).unwrap(),
            Value::Int(2)
        );
        // A#=2 -> Real(2.0)  (hw_verified iint_to_real).
        assert_eq!(
            Value::Int(2).coerce_to_suffix(Suffix::Real).unwrap(),
            Value::Real(2.0)
        );
    }

    #[test]
    fn coerce_no_suffix_keeps_runtime_type() {
        // A=2.7 stays Double (hw_verified nosuf_keepsreal); A=5 stays Integer.
        let r = Value::Real(2.7).coerce_to_suffix(Suffix::None).unwrap();
        assert_eq!(r.value_type(), ValueType::Real);
        let i = Value::Int(5).coerce_to_suffix(Suffix::None).unwrap();
        assert_eq!(i.value_type(), ValueType::Int);
    }

    #[test]
    fn coerce_across_string_numeric_divide_is_type_mismatch() {
        assert_eq!(
            Value::Int(1)
                .coerce_to_suffix(Suffix::Str)
                .unwrap_err()
                .errnum,
            8
        );
        assert_eq!(
            Value::str_from("x")
                .coerce_to_suffix(Suffix::None)
                .unwrap_err()
                .errnum,
            8
        );
        assert_eq!(
            Value::str_from("x")
                .coerce_to_suffix(Suffix::Int)
                .unwrap_err()
                .errnum,
            8
        );
    }

    #[test]
    fn defaults_match_suffix() {
        assert_eq!(Value::default_for_suffix(Suffix::None), Value::Int(0));
        assert_eq!(Value::default_for_suffix(Suffix::Int), Value::Int(0));
        assert_eq!(Value::default_for_suffix(Suffix::Real), Value::Real(0.0));
        assert_eq!(Value::default_for_suffix(Suffix::Str), Value::str_from(""));
    }

    #[test]
    fn references_alias_and_assign_through() {
        let cell = Value::cell(Value::Int(1));
        let r = Value::ref_to(&cell);
        assert_eq!(r.value_type(), ValueType::Ref);
        assert_eq!(r.deref(), Value::Int(1));
        r.assign_through(Value::Int(42)).unwrap();
        assert_eq!(*cell.borrow(), Value::Int(42));
        assert_eq!(r.deref(), Value::Int(42));
    }

    #[test]
    fn swap_exchanges_cells() {
        let a = Value::cell(Value::Int(1));
        let b = Value::cell(Value::str_from("hi"));
        swap_cells(&a, &b);
        assert_eq!(*a.borrow(), Value::str_from("hi"));
        assert_eq!(*b.borrow(), Value::Int(1));
        // aliased swap is a harmless no-op.
        swap_cells(&a, &a);
        assert_eq!(*a.borrow(), Value::str_from("hi"));
    }

    #[test]
    fn numeric_equality_crosses_int_real() {
        assert_eq!(Value::Int(2), Value::Real(2.0));
        assert_ne!(Value::Int(2), Value::Real(2.5));
        assert_ne!(Value::Int(2), Value::str_from("2"));
    }
}
