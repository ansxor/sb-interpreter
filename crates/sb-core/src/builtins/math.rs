//! Mathematics builtins (M1-T7) — `spec/instructions/{abs,floor,ceil,round,sgn,
//! classify,min,max,sqr,pow,exp,log,sin,cos,tan,asin,acos,atan,sinh,cosh,tanh,deg,
//! rad,pi}.yaml`.
//!
//! Type discipline follows the disassembled handlers: FLOOR/CEIL/ROUND/ABS **preserve**
//! the argument's numeric type (Integer in → Integer out; Double in → whole-valued
//! Double out), SGN/CLASSIFY always return an Integer, and SQR/POW/EXP/LOG/trig/DEG/RAD/
//! PI always return a Double. Numeric edge cases cited inline are `disassembled`
//! (FUN addresses in the specs); the documented values are `hw_verified`.

use super::{illegal, out_of_range, type_mismatch};
use crate::value::{RuntimeError, Value};

/// `1/PI` and `PI` as the doubles the DEG/RAD handlers load (`deg.yaml`/`rad.yaml`).
const PI: f64 = std::f64::consts::PI;
const DEG_PER_RAD: f64 = 180.0 / PI; // 57.29577951308232
const RAD_PER_DEG: f64 = PI / 180.0; // 0.017453292519943295

/// Require exactly one numeric argument, returning it.
fn one<'a>(args: &'a [Value], _name: &str) -> Result<&'a Value, RuntimeError> {
    match args {
        [v] => Ok(v),
        _ => Err(illegal()),
    }
}

/// Coerce a value to Double, mapping a string to Type mismatch (errnum 8).
fn real(v: &Value) -> Result<f64, RuntimeError> {
    v.to_real()
}

// -- type-preserving rounding family -------------------------------------------

/// `FLOOR(value)` — round toward −∞, preserving Integer/Double type.
pub(super) fn floor(args: &[Value]) -> Result<Value, RuntimeError> {
    match one(args, "FLOOR")? {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Real(d) => Ok(Value::Real(d.floor())),
        _ => Err(type_mismatch()),
    }
}

/// `CEIL(value)` — round toward +∞, preserving Integer/Double type.
pub(super) fn ceil(args: &[Value]) -> Result<Value, RuntimeError> {
    match one(args, "CEIL")? {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Real(d) => Ok(Value::Real(d.ceil())),
        _ => Err(type_mismatch()),
    }
}

/// `ROUND(value)` — nearest whole number, halves away from zero, preserving type.
/// Handler @0x144c50: `v>=0 → FLOOR(v+0.5)`, `v<0 → CEIL(v-0.5)` (`round.yaml`).
pub(super) fn round(args: &[Value]) -> Result<Value, RuntimeError> {
    match one(args, "ROUND")? {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Real(d) => {
            let r = if *d >= 0.0 {
                (d + 0.5).floor()
            } else {
                (d - 0.5).ceil()
            };
            Ok(Value::Real(r))
        }
        _ => Err(type_mismatch()),
    }
}

/// `ABS(value)` — absolute value, preserving type. The integer path is a reverse-subtract
/// with no saturation, so `ABS(INT_MIN)` wraps back to `INT_MIN` (`abs.yaml`).
pub(super) fn abs(args: &[Value]) -> Result<Value, RuntimeError> {
    match one(args, "ABS")? {
        Value::Int(i) => Ok(Value::Int(if *i < 0 { i.wrapping_neg() } else { *i })),
        Value::Real(d) => Ok(Value::Real(d.abs())),
        _ => Err(type_mismatch()),
    }
}

/// `SGN(value)` — −1/0/1, always Integer. NaN classifies as 1 (unordered VFP compare
/// falls through to the positive branch, `sgn.yaml`).
pub(super) fn sgn(args: &[Value]) -> Result<Value, RuntimeError> {
    let x = real(one(args, "SGN")?)?;
    let s = if x < 0.0 {
        -1
    } else if x == 0.0 {
        0
    } else {
        1 // x > 0 or NaN
    };
    Ok(Value::Int(s))
}

/// `CLASSIFY(value)` — 0 ordinary / 1 infinity / 2 NaN, always Integer. An Integer
/// argument always classifies as 0 without inspecting bits (`classify.yaml`).
pub(super) fn classify(args: &[Value]) -> Result<Value, RuntimeError> {
    match one(args, "CLASSIFY")? {
        Value::Int(_) => Ok(Value::Int(0)),
        Value::Real(d) => Ok(Value::Int(if d.is_nan() {
            2
        } else if d.is_infinite() {
            1
        } else {
            0
        })),
        _ => Err(type_mismatch()),
    }
}

/// `MIN(...)` / `MAX(...)` — the smallest/largest of either a numeric array's elements
/// (single array arg) or the directly-enumerated scalar values. Comparison is always in
/// Double, but the RESULT TYPE follows a per-form rule (hw_verified 2026-06-23 via
/// sb-oracle — `min.yaml`/`max.yaml`):
///
/// * **Array form** `MAX(arr)` (one array arg): the extreme *element*, keeping the
///   array's own Integer/Double element type (`MAX(Q%)` of `[7,3,1]` → Integer `7`, so
///   `MAX(Q%)*&H7FFFFFFF` int-wraps to `2147483641`). String array → Type mismatch (8).
/// * **Two scalar args** `MAX(a,b)`: standard numeric promotion — Integer iff *both* args
///   are Integer, otherwise Double (`MAX(7%,3)` → Integer; `MAX(7%,3.0)` → Double `7.0`).
///   This matches SmileBASIC compiling the 2-operand form inline.
/// * **Three or more scalar args** `MAX(a,b,c,…)`: **always Double**, regardless of arg
///   types (`MAX(7%,3,1)` → Double `7.0`, so `MAX(7%,3,1)*&H7FFFFFFF` floats to
///   `15032385529.0`). This matches the dispatched MAX/MIN handler (disasm
///   FUN_00148230 @0x148230: arg-count 0 → errnum 4 site @0x1483e0 `mov r0,#0x4`; scalar
///   path exits via the Double return `vldr.64 d0,[sp,#0x8]` @0x14833c).
pub(super) fn min_max(args: &[Value], want_max: bool) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(illegal()); // arg-count 0 → errnum 4 (FUN_00148230 @0x1483e0)
    }

    // Array form (single array arg): the extreme element, preserving element type.
    if args.len() == 1 && args[0].is_array() {
        let elems = array_elements(&args[0])?; // string array → Type mismatch (8)
        if elems.is_empty() {
            // An empty array has no element to return; the exact real-SB result is
            // queued (bd:sb-interpreter-5if).
            return Err(illegal());
        }
        return Ok(pick_extreme(&elems, want_max)?.clone());
    }

    // Single scalar arg is degenerate — return it (exact real-SB form queued).
    if args.len() == 1 {
        args[0].to_real()?; // string scalar → Type mismatch (8)
        return Ok(args[0].clone());
    }

    // Scalar forms with ≥2 args. The extreme is chosen by Double magnitude…
    let winner = pick_extreme(args, want_max)?;
    // …but the result TYPE is Integer only for exactly two all-Integer args; two mixed
    // args promote to Double, and three or more args are always Double.
    let all_int = args.iter().all(|a| matches!(a, Value::Int(_)));
    if args.len() == 2 && all_int {
        Ok(Value::Int(winner.to_int()?))
    } else {
        Ok(Value::Real(winner.to_real()?))
    }
}

/// The extreme (`max`/`min` by Double magnitude) of `elems`, returned by reference so the
/// caller can decide the result type. Ties keep the first occurrence. A string element
/// raises Type mismatch (errnum 8) via `to_real`.
fn pick_extreme(elems: &[Value], want_max: bool) -> Result<&Value, RuntimeError> {
    let mut best = &elems[0];
    let mut best_key = best.to_real()?;
    for v in &elems[1..] {
        let key = v.to_real()?;
        if (want_max && key > best_key) || (!want_max && key < best_key) {
            best = v;
            best_key = key;
        }
    }
    Ok(best)
}

/// The elements of a numeric array as scalar [`Value`]s, in row-major order. A string
/// array yields string elements (which MIN/MAX then reject via Type mismatch).
fn array_elements(v: &Value) -> Result<Vec<Value>, RuntimeError> {
    Ok(match v {
        Value::IntArray(a) => a
            .borrow()
            .as_slice()
            .iter()
            .map(|&i| Value::Int(i))
            .collect(),
        Value::RealArray(a) => a
            .borrow()
            .as_slice()
            .iter()
            .map(|&d| Value::Real(d))
            .collect(),
        Value::StrArray(a) => a
            .borrow()
            .as_slice()
            .iter()
            .map(|s| Value::Str(s.clone()))
            .collect(),
        _ => return Err(type_mismatch()),
    })
}

// -- always-Double functions ---------------------------------------------------

/// A 1-argument function whose argument coerces to Double and whose result is Double
/// (SIN/COS/SINH/COSH/TANH). Domain-free.
pub(super) fn unary_real(args: &[Value], f: fn(f64) -> f64) -> Result<Value, RuntimeError> {
    Ok(Value::Real(f(real(one(args, "fn")?)?)))
}

/// `SQR(value)` — non-negative square root (Double). `value < 0` → Out of range (10).
pub(super) fn sqr(args: &[Value]) -> Result<Value, RuntimeError> {
    let x = real(one(args, "SQR")?)?;
    if x < 0.0 {
        return Err(out_of_range());
    }
    Ok(Value::Real(x.sqrt()))
}

/// `TAN(value)` — tangent (Double). An infinite result raises Overflow (errnum 9)
/// instead of returning Inf (`tan.yaml`).
pub(super) fn tan(args: &[Value]) -> Result<Value, RuntimeError> {
    let r = real(one(args, "TAN")?)?.tan();
    if r.is_infinite() {
        return Err(RuntimeError::new(super::ERR_OVERFLOW));
    }
    Ok(Value::Real(r))
}

/// `ASIN(value)` — arc sine in [−π/2, π/2] (Double). Domain [−1, 1]; outside → Out of
/// range (10).
pub(super) fn asin(args: &[Value]) -> Result<Value, RuntimeError> {
    let x = real(one(args, "ASIN")?)?;
    if !(-1.0..=1.0).contains(&x) {
        return Err(out_of_range());
    }
    Ok(Value::Real(x.asin()))
}

/// `ACOS(value)` — arc cosine in [0, π] (Double). Domain [−1, 1]; outside → Out of
/// range (10).
pub(super) fn acos(args: &[Value]) -> Result<Value, RuntimeError> {
    let x = real(one(args, "ACOS")?)?;
    if !(-1.0..=1.0).contains(&x) {
        return Err(out_of_range());
    }
    Ok(Value::Real(x.acos()))
}

/// `ATAN(value)` / `ATAN(y, x)` — arc tangent (Double). One arg = `atan(value)`; two
/// args = `atan2(y, x)` with Y given first (`atan.yaml`).
pub(super) fn atan(args: &[Value]) -> Result<Value, RuntimeError> {
    match args {
        [v] => Ok(Value::Real(real(v)?.atan())),
        [y, x] => Ok(Value::Real(real(y)?.atan2(real(x)?))),
        _ => Err(illegal()),
    }
}

/// `POW(value, multiplier)` — `value ** multiplier` (Double). For `value < 0` the
/// exponent must be a whole number, else Illegal function call (4) (`pow.yaml`).
pub(super) fn pow(args: &[Value]) -> Result<Value, RuntimeError> {
    let (base, exp) = match args {
        [b, e] => (real(b)?, real(e)?),
        _ => return Err(illegal()),
    };
    if base < 0.0 && exp.fract() != 0.0 {
        return Err(illegal());
    }
    Ok(Value::Real(base.powf(exp)))
}

/// `EXP()` / `EXP(value)` — `e ** value` (Double). With no argument returns `e`
/// (`exp.yaml`).
pub(super) fn exp(args: &[Value]) -> Result<Value, RuntimeError> {
    match args {
        [] => Ok(Value::Real(std::f64::consts::E)),
        [v] => Ok(Value::Real(real(v)?.exp())),
        _ => Err(illegal()),
    }
}

/// `LOG(value)` / `LOG(value, base)` — natural log, or log in `base` (Double).
/// `value <= 0` or `base <= 0` → Out of range (10); `base == 1` → Illegal function
/// call (4) (`log.yaml`).
pub(super) fn log(args: &[Value]) -> Result<Value, RuntimeError> {
    match args {
        [v] => {
            let x = real(v)?;
            if x <= 0.0 {
                return Err(out_of_range());
            }
            Ok(Value::Real(x.ln()))
        }
        [v, b] => {
            let (x, base) = (real(v)?, real(b)?);
            if x <= 0.0 || base <= 0.0 {
                return Err(out_of_range());
            }
            if base == 1.0 {
                return Err(illegal());
            }
            Ok(Value::Real(x.ln() / base.ln()))
        }
        _ => Err(illegal()),
    }
}

/// `DEG(radians)` — radians → degrees (Double). No range check (`deg.yaml`).
pub(super) fn deg(args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Real(real(one(args, "DEG")?)? * DEG_PER_RAD))
}

/// `RAD(degrees)` — degrees → radians (Double). No range check (`rad.yaml`).
pub(super) fn rad(args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Real(real(one(args, "RAD")?)? * RAD_PER_DEG))
}

/// `PI()` — the circle constant π (Double). Takes no argument (`pi.yaml`).
pub(super) fn pi(args: &[Value]) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        Ok(Value::Real(PI))
    } else {
        Err(illegal())
    }
}

#[cfg(test)]
mod tests {
    use super::super::dispatch;
    use crate::array::SbArray;
    use crate::value::Value;

    /// Dispatch a builtin with literal args and unwrap the returned [`Value`].
    fn call(name: &str, args: Vec<Value>) -> Value {
        dispatch(name, args).unwrap()
    }
    /// Dispatch and expect an error, returning the errnum.
    fn err(name: &str, args: Vec<Value>) -> u32 {
        dispatch(name, args).unwrap_err().errnum
    }
    fn int(n: i32) -> Value {
        Value::Int(n)
    }
    fn real(x: f64) -> Value {
        Value::Real(x)
    }

    #[test]
    fn rounding_family_preserves_type() {
        // hw_verified docs values (floor/ceil/round.yaml).
        assert_eq!(call("FLOOR", vec![real(3.7)]), real(3.0));
        assert_eq!(call("FLOOR", vec![real(-2.1)]), real(-3.0));
        assert_eq!(call("FLOOR", vec![real(-12.5)]), real(-13.0));
        assert_eq!(call("FLOOR", vec![int(5)]), int(5)); // Integer passthrough
        assert_eq!(call("CEIL", vec![real(12.5)]), real(13.0));
        assert_eq!(call("CEIL", vec![real(-12.5)]), real(-12.0));
        assert_eq!(call("CEIL", vec![int(5)]), int(5));
        // ROUND half-away-from-zero.
        assert_eq!(call("ROUND", vec![real(2.5)]), real(3.0));
        assert_eq!(call("ROUND", vec![real(-2.5)]), real(-3.0));
        assert_eq!(call("ROUND", vec![real(0.5)]), real(1.0));
        assert_eq!(call("ROUND", vec![real(-0.5)]), real(-1.0));
        assert_eq!(call("ROUND", vec![real(2.4)]), real(2.0));
        assert_eq!(call("ROUND", vec![real(12.345)]), real(12.0));
    }

    #[test]
    fn rounding_family_stays_double_beyond_i32() {
        // Double input must stay Double (whole-valued) and NOT be coerced to Integer,
        // which would overflow/wrap for magnitudes above i32::MAX. This discriminates the
        // type-preserving return path documented in floor/ceil/round.yaml.
        const BIG: f64 = 3_000_000_000.0;
        const I32MAX_PLUS_1: f64 = 2_147_483_648.0;
        assert_eq!(call("FLOOR", vec![real(BIG)]), real(BIG));
        assert_eq!(call("FLOOR", vec![real(-BIG)]), real(-BIG));
        assert_eq!(
            call("FLOOR", vec![real(I32MAX_PLUS_1)]),
            real(I32MAX_PLUS_1)
        );
        assert_eq!(call("CEIL", vec![real(BIG)]), real(BIG));
        assert_eq!(call("CEIL", vec![real(-BIG)]), real(-BIG));
        assert_eq!(call("CEIL", vec![real(I32MAX_PLUS_1)]), real(I32MAX_PLUS_1));
        assert_eq!(call("ROUND", vec![real(BIG)]), real(BIG));
        assert_eq!(call("ROUND", vec![real(-BIG)]), real(-BIG));
        assert_eq!(
            call("ROUND", vec![real(I32MAX_PLUS_1)]),
            real(I32MAX_PLUS_1)
        );
        assert_eq!(call("ROUND", vec![real(BIG + 0.5)]), real(BIG + 1.0));
        // Sanity: Integer inputs still passthrough.
        assert_eq!(call("FLOOR", vec![int(i32::MAX)]), int(i32::MAX));
        assert_eq!(call("ROUND", vec![int(i32::MIN)]), int(i32::MIN));
    }

    #[test]
    fn abs_sgn_classify() {
        assert_eq!(call("ABS", vec![real(-12.345)]), real(12.345));
        assert_eq!(call("ABS", vec![int(-5)]), int(5));
        assert_eq!(call("ABS", vec![int(5)]), int(5));
        // INT_MIN reverse-subtract wraps (no saturation) — disassembled (abs.yaml).
        assert_eq!(call("ABS", vec![int(i32::MIN)]), int(i32::MIN));
        assert_eq!(call("SGN", vec![real(12.345)]), int(1));
        assert_eq!(call("SGN", vec![int(-5)]), int(-1));
        assert_eq!(call("SGN", vec![int(0)]), int(0));
        assert_eq!(call("SGN", vec![real(-0.0)]), int(0));
        assert_eq!(call("SGN", vec![real(f64::NAN)]), int(1)); // disassembled NaN→1
        assert_eq!(call("CLASSIFY", vec![real(0.5)]), int(0));
        assert_eq!(call("CLASSIFY", vec![int(5)]), int(0));
        assert_eq!(call("CLASSIFY", vec![real(f64::INFINITY)]), int(1));
        assert_eq!(call("CLASSIFY", vec![real(f64::NAN)]), int(2));
    }

    #[test]
    fn min_max_varargs_and_array() {
        // hw_verified 2026-06-23 (sb-oracle): result TYPE follows a per-form rule.
        // Two all-Integer scalar args → Integer (int-wraps under `*&H7FFFFFFF`).
        assert_eq!(call("MAX", vec![int(7), int(3)]), int(7));
        assert_eq!(call("MIN", vec![int(2), int(4)]), int(2));
        // Two mixed scalar args → Double (value preserved, retyped).
        assert_eq!(call("MIN", vec![int(3), real(2.5)]), real(2.5));
        assert_eq!(call("MAX", vec![int(3), real(2.5)]), real(3.0));
        // Three or more scalar args → ALWAYS Double, even all-Integer.
        assert_eq!(call("MAX", vec![int(7), int(3), int(1)]), real(7.0));
        assert_eq!(call("MIN", vec![int(1), int(2), int(3), int(4)]), real(1.0));
        assert_eq!(call("MIN", vec![int(-5), int(2), int(-10)]), real(-10.0));
        // Array form → keeps the array's element type (Integer array → Integer).
        let mut a = SbArray::<i32>::new(&[2]).unwrap();
        a.set(&[0], 50).unwrap();
        a.set(&[1], 3).unwrap();
        let arr = Value::IntArray(a.into_ref());
        assert_eq!(call("MIN", vec![arr.clone()]), int(3));
        assert_eq!(call("MAX", vec![arr]), int(50));
        // A Real array stays Double.
        let mut r = SbArray::<f64>::new(&[2]).unwrap();
        r.set(&[0], 2.5).unwrap();
        r.set(&[1], 9.5).unwrap();
        let rarr = Value::RealArray(r.into_ref());
        assert_eq!(call("MAX", vec![rarr]), real(9.5));
        // No args → Illegal function call.
        assert_eq!(err("MIN", vec![]), 4);
        // String element → Type mismatch.
        assert_eq!(err("MIN", vec![int(1), Value::str_from("x")]), 8);
    }

    #[test]
    fn powers_roots_logs() {
        assert_eq!(call("SQR", vec![int(4)]), real(2.0));
        assert_eq!(call("SQR", vec![int(0)]), real(0.0));
        assert_eq!(err("SQR", vec![int(-1)]), 10); // Out of range
        assert_eq!(call("POW", vec![int(2), int(10)]), real(1024.0));
        assert_eq!(call("POW", vec![int(2), int(-1)]), real(0.5));
        assert_eq!(call("POW", vec![int(-2), int(3)]), real(-8.0));
        assert_eq!(err("POW", vec![int(-2), real(0.5)]), 4); // illegal: frac exp on neg base
        assert_eq!(call("EXP", vec![int(0)]), real(1.0));
        assert_eq!(call("EXP", vec![]), real(std::f64::consts::E));
        assert_eq!(call("LOG", vec![int(1)]), real(0.0));
        assert_eq!(call("LOG", vec![int(8), int(2)]), real(3.0));
        assert_eq!(call("LOG", vec![int(100), int(10)]), real(2.0));
        // base in (0,1) allowed in 3.6.0 (LOG(8,0.5) = -3).
        assert_eq!(call("LOG", vec![int(8), real(0.5)]), real(-3.0));
        assert_eq!(err("LOG", vec![int(0)]), 10); // value<=0
        assert_eq!(err("LOG", vec![int(2), int(1)]), 4); // base==1
        assert_eq!(err("LOG", vec![int(2), int(0)]), 10); // base<=0
    }

    #[test]
    fn trig_and_angle() {
        assert_eq!(call("SIN", vec![int(0)]), real(0.0));
        assert_eq!(call("COS", vec![int(0)]), real(1.0));
        assert_eq!(call("TAN", vec![int(0)]), real(0.0));
        assert_eq!(call("ASIN", vec![int(0)]), real(0.0));
        assert_eq!(call("ACOS", vec![int(1)]), real(0.0));
        assert_eq!(call("ATAN", vec![int(0)]), real(0.0));
        // atan2(1,1) = pi/4.
        assert_eq!(
            call("ATAN", vec![int(1), int(1)]),
            real(std::f64::consts::FRAC_PI_4)
        );
        assert_eq!(err("ASIN", vec![int(2)]), 10); // domain
        assert_eq!(err("ACOS", vec![int(-2)]), 10);
        // DEG/RAD round-trip.
        assert_eq!(call("DEG", vec![real(super::PI)]), real(180.0));
        assert_eq!(call("RAD", vec![int(180)]), real(super::PI));
        assert_eq!(call("PI", vec![]), real(super::PI));
        assert_eq!(err("PI", vec![int(1)]), 4); // PI takes no arg
    }

    #[test]
    fn arg_count_and_type_errors() {
        assert_eq!(err("FLOOR", vec![]), 4);
        assert_eq!(err("FLOOR", vec![int(1), int(2)]), 4);
        assert_eq!(err("FLOOR", vec![Value::str_from("x")]), 8);
        assert_eq!(err("SQR", vec![Value::str_from("x")]), 8);
        assert_eq!(err("SIN", vec![Value::str_from("x")]), 8);
    }
}
