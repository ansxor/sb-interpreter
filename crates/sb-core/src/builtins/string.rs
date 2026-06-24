//! String builtins (M1-T7) — `spec/instructions/{len,left,right,mid,subst,str,val,
//! hex,format,asc,chr,instr}.yaml`.
//!
//! SmileBASIC strings are UTF-16 ([`SbStr`] = `Vec<u16>`), and all positions/lengths
//! are in **code units** (each full-width/kana character counts as one). Slicing is
//! therefore plain `u16`-slice indexing.

use super::{as_str, format_number, illegal, out_of_range, sb_string, type_mismatch};
use crate::value::{RuntimeError, SbStr, Value};

/// Width bound shared by STR$/HEX$ `digits` (`str.yaml`/`hex.yaml`: 1–63).
const MAX_FIELD_WIDTH: i32 = 63;

/// Coerce a value to an `i32` index/count, mapping a string to Type mismatch (8).
fn int(v: &Value) -> Result<i32, RuntimeError> {
    v.to_int()
}

/// `LEN(string)` / `LEN(array)` — character count of a string, or element count of an
/// array. A plain number is a Type mismatch (errnum 8) (`len.yaml`).
pub(super) fn len(args: &[Value]) -> Result<Value, RuntimeError> {
    let v = match args {
        [v] => v,
        _ => return Err(illegal()),
    };
    let n = match v {
        Value::Str(s) => s.len(),
        Value::IntArray(a) => a.borrow().len(),
        Value::RealArray(a) => a.borrow().len(),
        Value::StrArray(a) => a.borrow().len(),
        _ => return Err(type_mismatch()),
    };
    Ok(Value::Int(n as i32))
}

/// `LEFT$(string, length)` — leftmost `length` characters (clamped). Negative length →
/// Out of range (10) (`left.yaml`).
pub(super) fn left(args: &[Value]) -> Result<Value, RuntimeError> {
    let (s, n) = match args {
        [s, n] => (as_str(s)?, int(n)?),
        _ => return Err(illegal()),
    };
    if n < 0 {
        return Err(out_of_range());
    }
    let n = (n as usize).min(s.len());
    Ok(Value::Str(s[..n].to_vec()))
}

/// `RIGHT$(string, length)` — rightmost `length` characters (clamped). Negative length
/// → Out of range (10) (`right.yaml`).
pub(super) fn right(args: &[Value]) -> Result<Value, RuntimeError> {
    let (s, n) = match args {
        [s, n] => (as_str(s)?, int(n)?),
        _ => return Err(illegal()),
    };
    if n < 0 {
        return Err(out_of_range());
    }
    let n = (n as usize).min(s.len());
    Ok(Value::Str(s[s.len() - n..].to_vec()))
}

/// `MID$(string, start, length)` — `length` characters from the 0-based `start`, both
/// clamped so the slice never runs past the end (`mid.yaml`).
pub(super) fn mid(args: &[Value]) -> Result<Value, RuntimeError> {
    let (s, start, length) = match args {
        [s, a, b] => (as_str(s)?, int(a)?, int(b)?),
        _ => return Err(illegal()),
    };
    let len = s.len();
    let start = start.max(0).min(len as i32) as usize;
    let length = length.max(0) as usize;
    let end = start.saturating_add(length).min(len);
    Ok(Value::Str(s[start..end].to_vec()))
}

/// `SUBST$(string, start, replacement)` or `SUBST$(string, start, count, replacement)`
/// — `string[0:start] + replacement + string[start+count:]`. The 3-arg form replaces
/// from `start` to the end (`count = len - start`) (`subst.yaml`).
pub(super) fn subst(args: &[Value]) -> Result<Value, RuntimeError> {
    let (s, start, count, repl) = match args {
        [s, a, r] => {
            let s = as_str(s)?;
            let start = int(a)?.max(0).min(s.len() as i32) as usize;
            let count = s.len() - start;
            (s, start, count, as_str(r)?)
        }
        [s, a, c, r] => {
            let s = as_str(s)?;
            let start = int(a)?.max(0).min(s.len() as i32) as usize;
            let count = (int(c)?.max(0) as usize).min(s.len() - start);
            (s, start, count, as_str(r)?)
        }
        _ => return Err(illegal()),
    };
    let mut out: SbStr = Vec::with_capacity(s.len() + repl.len());
    out.extend_from_slice(&s[..start]);
    out.extend_from_slice(repl);
    out.extend_from_slice(&s[start + count..]);
    Ok(Value::Str(out))
}

/// `STR$(value)` / `STR$(value, digits)` — decimal string form, optionally right-
/// justified to a minimum field width. `digits` outside 1–63 → Out of range (10)
/// (`str.yaml`).
pub(super) fn str_(args: &[Value]) -> Result<Value, RuntimeError> {
    let (v, digits) = match args {
        [v] => (v, None),
        [v, d] => (v, Some(int(d)?)),
        _ => return Err(illegal()),
    };
    let body = format_number(v)?;
    match digits {
        None => Ok(sb_string(&body)),
        Some(w) => {
            if !(1..=MAX_FIELD_WIDTH).contains(&w) {
                return Err(out_of_range());
            }
            Ok(sb_string(&right_justify(&body, w as usize)))
        }
    }
}

/// `HEX$(value)` / `HEX$(value, digits)` — uppercase 32-bit hex (two's-complement for
/// negatives), optionally left zero-padded to `digits`. `digits` outside 1–63 → Out of
/// range (10) (`hex.yaml`).
pub(super) fn hex(args: &[Value]) -> Result<Value, RuntimeError> {
    let (v, digits) = match args {
        [v] => (v, None),
        [v, d] => (v, Some(int(d)?)),
        _ => return Err(illegal()),
    };
    let bits = int(v)? as u32;
    let body = format!("{bits:X}");
    match digits {
        None => Ok(sb_string(&body)),
        Some(w) => {
            if !(1..=MAX_FIELD_WIDTH).contains(&w) {
                return Err(out_of_range());
            }
            Ok(sb_string(&format!("{bits:0>width$X}", width = w as usize)))
        }
    }
}

/// `ASC(string)` — UTF-16 code of the first character. Empty string → Illegal function
/// call (4); non-string → Type mismatch (8) (`asc.yaml`).
pub(super) fn asc(args: &[Value]) -> Result<Value, RuntimeError> {
    let s = match args {
        [s] => as_str(s)?,
        _ => return Err(illegal()),
    };
    match s.first() {
        Some(&u) => Ok(Value::Int(u as i32)),
        None => Err(illegal()),
    }
}

/// `CHR$(code)` — one-character string from the low 16 bits of `code` (`chr.yaml`).
pub(super) fn chr(args: &[Value]) -> Result<Value, RuntimeError> {
    let code = match args {
        [c] => int(c)?,
        _ => return Err(illegal()),
    };
    Ok(Value::Str(vec![code as u16]))
}

/// `INSTR(haystack, needle)` or `INSTR(start, haystack, needle)` — 0-based index of the
/// first match at/after `start` (default 0), or −1. An empty needle matches at `start`
/// (`instr.yaml`).
pub(super) fn instr(args: &[Value]) -> Result<Value, RuntimeError> {
    let (start, haystack, needle) = match args {
        [h, n] => (0i32, as_str(h)?, as_str(n)?),
        [s, h, n] => (int(s)?, as_str(h)?, as_str(n)?),
        _ => return Err(illegal()),
    };
    let start = start.max(0) as usize;
    if start > haystack.len() {
        return Ok(Value::Int(-1));
    }
    if needle.is_empty() {
        return Ok(Value::Int(start as i32));
    }
    let pos = haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle.as_slice())
        .map(|p| (start + p) as i32)
        .unwrap_or(-1);
    Ok(Value::Int(pos))
}

/// `VAL(string)` — parse the leading numeric portion. The *whole* (trimmed) string must
/// be a valid number, else the result is 0; supports `&H`/`&B`/exponent forms. Returns
/// an Integer for a plain integer literal, a Double when a fraction/exponent is present
/// (`val.yaml`). A non-string argument is a Type mismatch (errnum 8).
pub(super) fn val(args: &[Value]) -> Result<Value, RuntimeError> {
    let s = match args {
        [s] => as_str(s)?,
        _ => return Err(illegal()),
    };
    let text = String::from_utf16_lossy(s);
    let t = text.trim();
    if t.is_empty() {
        return Ok(Value::Int(0));
    }
    // `&H` hex / `&B` binary → 32-bit integer (wrapping).
    if let Some(hex) = strip_prefix_ci(t, "&H") {
        return Ok(match u32::from_str_radix(hex, 16) {
            Ok(u) => Value::Int(u as i32),
            Err(_) => Value::Int(0),
        });
    }
    if let Some(bin) = strip_prefix_ci(t, "&B") {
        return Ok(match u32::from_str_radix(bin, 2) {
            Ok(u) => Value::Int(u as i32),
            Err(_) => Value::Int(0),
        });
    }
    // A plain integer (no fraction/exponent) keeps Integer type.
    if !t.contains(['.', 'e', 'E']) {
        if let Ok(i) = t.parse::<i32>() {
            return Ok(Value::Int(i));
        }
    }
    match t.parse::<f64>() {
        Ok(f) if f.is_finite() => Ok(Value::Real(f)),
        _ => Ok(Value::Int(0)),
    }
}

/// `FORMAT$(format, values...)` — printf-style formatting. Directives `%S` `%D` `%X`
/// `%F` `%B` consume the next value in order; `%%` emits a literal `%` (`format.yaml`).
/// A non-string `format`, or a value whose type doesn't match its directive, is a Type
/// mismatch (errnum 8).
pub(super) fn format(args: &[Value]) -> Result<Value, RuntimeError> {
    let fmt = match args.first() {
        Some(v) => as_str(v)?,
        None => return Err(illegal()),
    };
    let rest = &args[1..];
    let chars: Vec<char> = String::from_utf16_lossy(fmt).chars().collect();
    let mut out = String::new();
    let mut ai = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c != '%' {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        if chars.get(i) == Some(&'%') {
            out.push('%');
            i += 1;
            continue;
        }
        // Flags.
        let (mut left, mut plus, mut space, mut zero) = (false, false, false, false);
        while let Some(&f) = chars.get(i) {
            match f {
                '-' => left = true,
                '+' => plus = true,
                ' ' => space = true,
                '0' => zero = true,
                _ => break,
            }
            i += 1;
        }
        // Width.
        let mut width = 0usize;
        while let Some(d) = chars.get(i).and_then(|c| c.to_digit(10)) {
            width = width * 10 + d as usize;
            i += 1;
        }
        // Precision.
        let mut prec: Option<usize> = None;
        if chars.get(i) == Some(&'.') {
            i += 1;
            let mut p = 0usize;
            while let Some(d) = chars.get(i).and_then(|c| c.to_digit(10)) {
                p = p * 10 + d as usize;
                i += 1;
            }
            prec = Some(p);
        }
        let conv = match chars.get(i) {
            Some(&c) => c,
            None => break, // trailing `%…` with no conversion: stop (ill-formed)
        };
        i += 1;
        let arg = rest.get(ai).ok_or_else(illegal)?;
        ai += 1;
        let sign = |neg: bool| -> &'static str {
            if neg {
                "-"
            } else if plus {
                "+"
            } else if space {
                " "
            } else {
                ""
            }
        };
        match conv.to_ascii_uppercase() {
            'D' => {
                let n = int(arg)?;
                push_numeric(
                    &mut out,
                    sign(n < 0),
                    &n.unsigned_abs().to_string(),
                    width,
                    left,
                    zero,
                );
            }
            'X' => {
                let bits = int(arg)? as u32;
                push_numeric(&mut out, "", &format!("{bits:X}"), width, left, zero);
            }
            'B' => {
                let bits = int(arg)? as u32;
                push_numeric(&mut out, "", &format!("{bits:b}"), width, left, zero);
            }
            'F' => {
                let x = arg.to_real()?;
                let p = prec.unwrap_or(6);
                push_numeric(
                    &mut out,
                    sign(x.is_sign_negative()),
                    &format!("{:.p$}", x.abs(), p = p),
                    width,
                    left,
                    zero,
                );
            }
            'S' => {
                let s = String::from_utf16_lossy(as_str(arg)?);
                // String fields pad with spaces only (the zero flag is ignored).
                push_numeric(&mut out, "", &s, width, left, false);
            }
            other => {
                // Unknown directive: emit it verbatim and don't consume the arg.
                ai -= 1;
                out.push('%');
                out.push(other);
            }
        }
    }
    Ok(sb_string(&out))
}

// -- formatting helpers --------------------------------------------------------

/// Right-justify `body` to `width` with leading spaces (no truncation).
fn right_justify(body: &str, width: usize) -> String {
    if body.len() >= width {
        body.to_string()
    } else {
        format!("{}{}", " ".repeat(width - body.len()), body)
    }
}

/// Append a (possibly signed) field to `out`, applying width/left/zero padding. Zero
/// padding goes between the sign and the digits and is ignored when left-aligning.
fn push_numeric(out: &mut String, sign: &str, digits: &str, width: usize, left: bool, zero: bool) {
    let total = sign.len() + digits.len();
    if total >= width {
        out.push_str(sign);
        out.push_str(digits);
        return;
    }
    let pad = width - total;
    if left {
        out.push_str(sign);
        out.push_str(digits);
        out.push_str(&" ".repeat(pad));
    } else if zero {
        out.push_str(sign);
        out.push_str(&"0".repeat(pad));
        out.push_str(digits);
    } else {
        out.push_str(&" ".repeat(pad));
        out.push_str(sign);
        out.push_str(digits);
    }
}

/// Strip a case-insensitive ASCII prefix, returning the remainder if it matched.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    // `get(..n)` is `None` when `n` is past the end OR lands inside a multi-byte char, so a
    // SmileBASIC string whose first character is a full-width/kana glyph (e.g. `VAL("\u{ffde}")`)
    // can never byte-slice mid-codepoint here — a fuzzer-found panic (M7-T1).
    let head = s.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    // These are documented SmileBASIC test values (e.g. STR$(3.14159265), VAL("3.14")),
    // not approximations of π — silence the approx-constant lint for the literals.
    #![allow(clippy::approx_constant)]
    use super::super::dispatch;
    use crate::value::Value;

    /// Dispatch and unwrap to a Rust `String` (the value must be a string).
    fn s(name: &str, args: Vec<Value>) -> String {
        match dispatch(name, args).unwrap() {
            Value::Str(u) => String::from_utf16_lossy(&u),
            other => panic!("expected a string, got {other:?}"),
        }
    }
    /// Dispatch and unwrap to an `i32` (the value must be an Integer).
    fn n(name: &str, args: Vec<Value>) -> i32 {
        match dispatch(name, args).unwrap() {
            Value::Int(i) => i,
            other => panic!("expected an Integer, got {other:?}"),
        }
    }
    fn err(name: &str, args: Vec<Value>) -> u32 {
        dispatch(name, args).unwrap_err().errnum
    }
    fn st(text: &str) -> Value {
        Value::str_from(text)
    }
    fn i(v: i32) -> Value {
        Value::Int(v)
    }

    #[test]
    fn extract_family() {
        assert_eq!(s("LEFT$", vec![st("ABC"), i(5)]), "ABC"); // clamped
        assert_eq!(s("LEFT$", vec![st("ABC"), i(0)]), "");
        assert_eq!(s("RIGHT$", vec![st("ABCDEF"), i(2)]), "EF");
        assert_eq!(s("RIGHT$", vec![st("ABC"), i(5)]), "ABC");
        assert_eq!(s("MID$", vec![st("ABC"), i(0), i(2)]), "AB");
        assert_eq!(s("MID$", vec![st("ABC"), i(1), i(2)]), "BC");
        assert_eq!(s("MID$", vec![st("ABC"), i(1), i(5)]), "BC"); // length clamped
        assert_eq!(s("MID$", vec![st("ABC"), i(5), i(1)]), ""); // start past end
        assert_eq!(err("LEFT$", vec![st("ABC"), i(-1)]), 10); // negative length
        assert_eq!(err("RIGHT$", vec![st("ABC"), i(-1)]), 10);
    }

    #[test]
    fn subst_forms() {
        assert_eq!(s("SUBST$", vec![st("ABC"), i(0), i(2), st("XY")]), "XYC");
        assert_eq!(s("SUBST$", vec![st("ABC"), i(0), i(0), st("X")]), "XABC"); // insert
                                                                               // 3-arg form replaces start..end.
        assert_eq!(s("SUBST$", vec![st("ABCDEF"), i(2), st("XY")]), "ABXY");
        // replacement may grow/shrink the string.
        assert_eq!(s("SUBST$", vec![st("ABC"), i(1), i(1), st("ZZZ")]), "AZZZC");
    }

    #[test]
    fn len_string_and_array() {
        assert_eq!(n("LEN", vec![st("ABC123")]), 6);
        assert_eq!(n("LEN", vec![st("")]), 0);
        let mut a = crate::array::SbArray::<i32>::new(&[4]).unwrap();
        a.set(&[0], 9).unwrap();
        assert_eq!(n("LEN", vec![Value::IntArray(a.into_ref())]), 4);
        assert_eq!(err("LEN", vec![i(5)]), 8); // plain number → Type mismatch
    }

    #[test]
    fn asc_chr_instr() {
        assert_eq!(n("ASC", vec![st("A")]), 65);
        assert_eq!(n("ASC", vec![st("AB")]), 65); // first char only
        assert_eq!(err("ASC", vec![st("")]), 4); // empty → illegal
        assert_eq!(err("ASC", vec![i(5)]), 8); // non-string
        assert_eq!(s("CHR$", vec![i(65)]), "A");
        assert_eq!(s("CHR$", vec![i(0x3042)]), "\u{3042}"); // hiragana あ
        assert_eq!(n("INSTR", vec![st("ABCDEF"), st("CD")]), 2);
        assert_eq!(n("INSTR", vec![st("ABC"), st("X")]), -1);
        assert_eq!(n("INSTR", vec![i(2), st("ABAB"), st("A")]), 2);
        assert_eq!(n("INSTR", vec![st("ABC"), st("")]), 0); // empty needle at start
    }

    #[test]
    fn str_conversion_and_width() {
        assert_eq!(s("STR$", vec![i(123)]), "123");
        assert_eq!(s("STR$", vec![i(-5)]), "-5");
        assert_eq!(s("STR$", vec![i(123), i(6)]), "   123"); // right-justified width
        assert_eq!(s("STR$", vec![Value::Real(3.14159265)]), "3.14159");
        assert_eq!(s("STR$", vec![Value::Real(0.5)]), "0.5");
        assert_eq!(s("STR$", vec![Value::Real(0.00001)]), "1e-05");
        assert_eq!(s("STR$", vec![Value::Real(12345678.0)]), "1.23457e+07");
        assert_eq!(err("STR$", vec![st("x")]), 8); // value is a string
        assert_eq!(err("STR$", vec![i(1), i(0)]), 10); // digits out of 1..63
        assert_eq!(err("STR$", vec![i(1), i(64)]), 10);
    }

    #[test]
    fn hex_conversion() {
        assert_eq!(s("HEX$", vec![i(255)]), "FF");
        assert_eq!(s("HEX$", vec![i(16)]), "10");
        assert_eq!(s("HEX$", vec![i(-1)]), "FFFFFFFF"); // two's-complement
        assert_eq!(s("HEX$", vec![i(255), i(4)]), "00FF"); // zero-padded
        assert_eq!(s("HEX$", vec![i(65535), i(4)]), "FFFF");
        assert_eq!(err("HEX$", vec![i(1), i(0)]), 10);
    }

    #[test]
    fn val_parsing() {
        assert_eq!(dispatch("VAL", vec![st("123")]).unwrap(), Value::Int(123));
        assert_eq!(dispatch("VAL", vec![st("-5")]).unwrap(), Value::Int(-5));
        assert_eq!(
            dispatch("VAL", vec![st("3.14")]).unwrap(),
            Value::Real(3.14)
        );
        assert_eq!(dispatch("VAL", vec![st("12ABC")]).unwrap(), Value::Int(0));
        assert_eq!(dispatch("VAL", vec![st("ABC")]).unwrap(), Value::Int(0));
        assert_eq!(dispatch("VAL", vec![st("")]).unwrap(), Value::Int(0));
        assert_eq!(
            dispatch("VAL", vec![st("1E3")]).unwrap(),
            Value::Real(1000.0)
        );
        assert_eq!(dispatch("VAL", vec![st("&HFF")]).unwrap(), Value::Int(255));
        assert_eq!(dispatch("VAL", vec![st("&B1010")]).unwrap(), Value::Int(10));
        assert_eq!(err("VAL", vec![i(5)]), 8); // non-string
    }

    #[test]
    fn format_directives() {
        assert_eq!(s("FORMAT$", vec![st("%D"), i(42)]), "42");
        assert_eq!(s("FORMAT$", vec![st("%06D"), i(42)]), "000042");
        assert_eq!(s("FORMAT$", vec![st("%X"), i(255)]), "FF");
        assert_eq!(s("FORMAT$", vec![st("%4X"), i(255)]), "  FF");
        assert_eq!(
            s("FORMAT$", vec![st("%8.2F"), Value::Real(3.14159)]),
            "    3.14"
        );
        assert_eq!(s("FORMAT$", vec![st("%S"), st("HI")]), "HI");
        assert_eq!(s("FORMAT$", vec![st("%+D"), i(5)]), "+5");
        assert_eq!(s("FORMAT$", vec![st("%-6D"), i(42)]), "42    ");
        assert_eq!(s("FORMAT$", vec![st("V=%D"), i(7)]), "V=7");
        assert_eq!(s("FORMAT$", vec![st("%D%%"), i(50)]), "50%");
        assert_eq!(s("FORMAT$", vec![st("%04B"), i(10)]), "1010");
        assert_eq!(err("FORMAT$", vec![i(5), i(1)]), 8); // non-string format
    }
}
