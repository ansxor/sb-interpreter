//! Builtin functions (M1-T7) — the registered native functions the VM dispatches
//! [`Op::CallBuiltin`](crate::bytecode::Op::CallBuiltin) to.
//!
//! This slice covers the **Mathematics** (`spec/instructions/{abs,floor,ceil,round,
//! sgn,classify,min,max,sqr,pow,exp,log,sin,cos,tan,asin,acos,atan,sinh,cosh,tanh,
//! deg,rad,pi}.yaml`) and **Strings** (`{len,left,right,mid,subst,str,val,hex,
//! format,asc,chr,instr}.yaml`) categories. RNG (`RND`/`RNDF`/`RANDOMIZE`, M1-T9) and
//! the console/command builtins (M1-T8+) land in later slices.
//!
//! ## Calling convention
//!
//! The compiler pushes a builtin's value args left-to-right (topmost = last arg), then
//! emits `CallBuiltin { name, argc, .. }`. [`dispatch`] receives the args in
//! source order and returns the function's value. These math/string builtins are pure
//! functions: they take no `OUT` params and produce exactly one return value (the VM
//! discards it when the call is in statement position).
//!
//! ## Errors
//!
//! Per the specs, the shared error numbers are: **Illegal function call** (4) for a bad
//! argument *count*, **Type mismatch** (8) for a bad argument *type* (string where a
//! number is wanted or vice-versa), **Overflow** (9), and **Out of range** (10). An
//! unregistered name raises **Undefined function** (16) (the registry below is the
//! authority on what is a builtin in this slice).
//!
//! Number→string formatting ([`format_number`]) follows the disassembled STR$ contract:
//! integers via C `%d`, doubles via C `%g` to 6 significant figures (the exact
//! tie-breaking / very-large-magnitude edges are M7-T4; see `HARVEST_QUEUE.md`).

pub(crate) mod console;
mod math;
mod string;

use crate::value::{RuntimeError, SbStr, Value};

// errnums shared across the builtins (names per `spec/reference/errors.yaml`).
pub(crate) const ERR_ILLEGAL_FUNCTION_CALL: u32 = 4;
pub(crate) const ERR_TYPE_MISMATCH: u32 = 8;
pub(crate) const ERR_OVERFLOW: u32 = 9;
pub(crate) const ERR_OUT_OF_RANGE: u32 = 10;
const ERR_UNDEFINED_FUNCTION: u32 = 16;

/// Build an `Illegal function call` (errnum 4) — a wrong argument count.
pub(crate) fn illegal() -> RuntimeError {
    RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL)
}
/// Build a `Type mismatch` (errnum 8) — a wrong argument type.
pub(crate) fn type_mismatch() -> RuntimeError {
    RuntimeError::new(ERR_TYPE_MISMATCH)
}
/// Build an `Out of range` (errnum 10).
pub(crate) fn out_of_range() -> RuntimeError {
    RuntimeError::new(ERR_OUT_OF_RANGE)
}

/// Canonical names of every builtin implemented in this slice (Mathematics + Strings).
/// The string-returning functions keep their `$` suffix, matching the compiler's
/// [`canonical`](crate::compiler) form.
pub const BUILTIN_NAMES: &[&str] = &[
    // Mathematics
    "ABS",
    "FLOOR",
    "CEIL",
    "ROUND",
    "SGN",
    "CLASSIFY",
    "MIN",
    "MAX",
    "SQR",
    "POW",
    "EXP",
    "LOG",
    "SIN",
    "COS",
    "TAN",
    "ASIN",
    "ACOS",
    "ATAN",
    "SINH",
    "COSH",
    "TANH",
    "DEG",
    "RAD",
    "PI",
    // RNG (M1-T9; handled in the VM since they mutate the TinyMT series state)
    "RND",
    "RNDF",
    "RANDOMIZE",
    // Strings
    "LEN",
    "LEFT$",
    "RIGHT$",
    "MID$",
    "SUBST$",
    "STR$",
    "VAL",
    "HEX$",
    "FORMAT$",
    "ASC",
    "CHR$",
    "INSTR",
    // Console I/O (M1-T8; LOCATE/COLOR/CLS/ACLS/BACKCOLOR mutate console/screen state and
    // INKEY$/INPUT/LINPUT read input, so the VM handles them directly rather than through
    // the stateless `dispatch`).
    "LOCATE",
    "COLOR",
    "CLS",
    "ACLS",
    "BACKCOLOR",
    "INKEY$",
];

/// The set of builtins known to the compiler in this slice, so a bare-name use (`PI`) or
/// paren call (`SQR(2)`) compiles as a [`CallBuiltin`](crate::bytecode::Op::CallBuiltin)
/// rather than a variable. Pass it to
/// [`compile_with`](crate::compiler::compile_with).
#[derive(Debug, Clone, Copy, Default)]
pub struct StdBuiltins;

impl crate::compiler::Builtins for StdBuiltins {
    fn is_builtin(&self, name: &str) -> bool {
        BUILTIN_NAMES.contains(&name)
    }
}

/// Dispatch a builtin call. `name` is the canonical (uppercased, `$`-kept) function
/// name; `args` are the value arguments in source order. Returns the function's value.
/// An unregistered name raises Undefined function (errnum 16).
pub fn dispatch(name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Defensively read through any reference arg (refs shouldn't reach here, but a
    // builtin always wants the pointed-to value, never the cell).
    let args: Vec<Value> = args.into_iter().map(|v| v.deref()).collect();
    match name {
        // -- Mathematics -------------------------------------------------------
        "ABS" => math::abs(&args),
        "FLOOR" => math::floor(&args),
        "CEIL" => math::ceil(&args),
        "ROUND" => math::round(&args),
        "SGN" => math::sgn(&args),
        "CLASSIFY" => math::classify(&args),
        "MIN" => math::min_max(&args, false),
        "MAX" => math::min_max(&args, true),
        "SQR" => math::sqr(&args),
        "POW" => math::pow(&args),
        "EXP" => math::exp(&args),
        "LOG" => math::log(&args),
        "SIN" => math::unary_real(&args, f64::sin),
        "COS" => math::unary_real(&args, f64::cos),
        "TAN" => math::tan(&args),
        "ASIN" => math::asin(&args),
        "ACOS" => math::acos(&args),
        "ATAN" => math::atan(&args),
        "SINH" => math::unary_real(&args, f64::sinh),
        "COSH" => math::unary_real(&args, f64::cosh),
        "TANH" => math::unary_real(&args, f64::tanh),
        "DEG" => math::deg(&args),
        "RAD" => math::rad(&args),
        "PI" => math::pi(&args),
        // -- Strings -----------------------------------------------------------
        "LEN" => string::len(&args),
        "LEFT$" => string::left(&args),
        "RIGHT$" => string::right(&args),
        "MID$" => string::mid(&args),
        "SUBST$" => string::subst(&args),
        "STR$" => string::str_(&args),
        "VAL" => string::val(&args),
        "HEX$" => string::hex(&args),
        "FORMAT$" => string::format(&args),
        "ASC" => string::asc(&args),
        "CHR$" => string::chr(&args),
        "INSTR" => string::instr(&args),
        _ => Err(RuntimeError::new(ERR_UNDEFINED_FUNCTION)),
    }
}

// -- number formatting (shared by STR$ / FORMAT$ / a future PRINT) -------------

/// Format a numeric [`Value`] to its SmileBASIC string form: an Integer via C `%d`,
/// a Double via C `%g` at 6 significant figures (disassembled STR$ contract). A
/// non-numeric value raises Type mismatch (errnum 8).
pub fn format_number(v: &Value) -> Result<String, RuntimeError> {
    match v {
        Value::Int(i) => Ok(i.to_string()),
        Value::Real(d) => Ok(format_g(*d, 6)),
        _ => Err(type_mismatch()),
    }
}

/// C `printf("%g")`-style formatting at `prec` significant figures: fixed notation
/// unless the decimal exponent is `< -4` or `>= prec`, in which case lowercase
/// exponential notation with a signed ≥2-digit exponent; trailing zeros (and a bare
/// trailing `.`) are stripped. Mirrors the doubles SB prints (e.g. `1.23457e+07`,
/// `1e-05`, `3.14159`). Exact half-way rounding / huge-magnitude edges are M7-T4.
pub fn format_g(x: f64, prec: usize) -> String {
    let prec = prec.max(1);
    if x == 0.0 {
        return "0".to_string();
    }
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    // Decompose with `prec-1` fractional digits to learn the decimal exponent.
    let sci = format!("{:.*e}", prec - 1, x); // e.g. "1.23457e7" / "-5e0"
    let (mantissa, exp_part) = sci.split_once('e').expect("`{:e}` always has an exponent");
    let exp: i32 = exp_part.parse().expect("`{:e}` exponent is an integer");
    if exp < -4 || exp >= prec as i32 {
        let mantissa = trim_fraction(mantissa);
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{mantissa}e{sign}{:02}", exp.abs())
    } else {
        // Fixed notation: enough fractional digits to keep `prec` significant figures.
        let frac_digits = (prec as i32 - 1 - exp).max(0) as usize;
        trim_fraction(&format!("{x:.frac_digits$}"))
    }
}

/// Strip trailing zeros after a decimal point, then a bare trailing `.`.
fn trim_fraction(s: &str) -> String {
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        trimmed.trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// Encode a Rust `&str` as a SmileBASIC UTF-16 string value.
pub(crate) fn sb_string(s: &str) -> Value {
    Value::Str(s.encode_utf16().collect())
}

/// The UTF-16 code units of a string [`Value`], or Type mismatch (errnum 8).
pub(crate) fn as_str(v: &Value) -> Result<&SbStr, RuntimeError> {
    match v {
        Value::Str(s) => Ok(s),
        _ => Err(type_mismatch()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_g_matches_smilebasic_doubles() {
        // hw_verified expects from the math specs (sb-oracle 2026-06-22).
        assert_eq!(format_g(std::f64::consts::PI, 6), "3.14159");
        assert_eq!(format_g(std::f64::consts::E, 6), "2.71828");
        assert_eq!(format_g(2.0_f64.sqrt(), 6), "1.41421");
        assert_eq!(format_g(0.5, 6), "0.5");
        assert_eq!(format_g(180.0, 6), "180");
        assert_eq!(format_g(0.00001, 6), "1e-05");
        assert_eq!(format_g(12345678.0, 6), "1.23457e+07");
        assert_eq!(format_g(0.0, 6), "0");
        assert_eq!(format_g(-0.0, 6), "0");
        assert_eq!(format_g(std::f64::consts::TAU, 6), "6.28319");
    }

    #[test]
    fn format_number_int_and_real() {
        assert_eq!(format_number(&Value::Int(123)).unwrap(), "123");
        assert_eq!(format_number(&Value::Int(-5)).unwrap(), "-5");
        assert_eq!(format_number(&Value::Real(3.0)).unwrap(), "3");
        assert_eq!(
            format_number(&sb_string("x")).unwrap_err().errnum,
            ERR_TYPE_MISMATCH
        );
    }

    #[test]
    fn unknown_name_is_undefined_function() {
        assert_eq!(
            dispatch("NOPE", vec![]).unwrap_err().errnum,
            ERR_UNDEFINED_FUNCTION
        );
    }

    #[test]
    fn std_builtins_predicate_covers_the_set() {
        use crate::compiler::Builtins;
        let b = StdBuiltins;
        assert!(b.is_builtin("FLOOR"));
        assert!(b.is_builtin("MID$"));
        assert!(b.is_builtin("PI"));
        assert!(b.is_builtin("RND")); // M1-T9
        assert!(b.is_builtin("RANDOMIZE")); // M1-T9
        assert!(!b.is_builtin("SPSET")); // later milestone
    }
}
