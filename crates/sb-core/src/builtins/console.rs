//! Console builtins (M1-T8) — the SmileBASIC text-console commands the VM drives over
//! the [`Console`](sb_render::console::Console) model it owns (M1-T10).
//!
//! These are the **console I/O** commands `LOCATE`/`COLOR`/`CLS`/`ACLS`, the non-blocking
//! `INKEY$` function, and the grid-reader `CHKCHR`. Unlike the math/string builtins, they
//! touch console state, so the VM routes them here with a `Console` (mutable for the
//! commands, shared for the readers) rather than through the stateless
//! [`dispatch`](super::dispatch). `PRINT` (its own opcodes), `BACKCOLOR` and
//! `INPUT`/`LINPUT` (which also need VM-level screen/input state) are handled in the VM.
//!
//! Argument-count / range contracts come straight from the disassembled handlers
//! (`spec/instructions/{locate,color,cls,acls,inkey}.yaml`), most error cases
//! `hw_verified`:
//!
//! - `LOCATE x,y[,z]` — 2 or 3 slots (else errnum 4); X 0..50, Y 0..29, Z -256.0..1024.0
//!   (else errnum 10). An omitted X/Y/Z (a comma placeholder → [`Value::Void`]) keeps the
//!   previous value.
//! - `COLOR fg[,bg]` — 1 or 2 slots (else errnum 4); each index 0..15 (else errnum 10).
//! - `CLS` — no arguments (else errnum 4).
//! - `ACLS` — 0 or 3 arguments (else errnum 4); resets the console draw state.
//! - `INKEY$()` — no arguments (else errnum 4); returns one buffered key or `""` (always
//!   `""` headless: there is no live keyboard buffer here).
//! - `CHKCHR(x,y)` — function only, exactly 2 args (else errnum 4); returns the cell's
//!   UTF-16 code, or 0 for an empty / out-of-bounds coordinate (no error).
//!
//! Using `LOCATE`/`COLOR`/`CLS`/`ACLS` as a function (requesting a return value) is the
//! errnum-4 misuse the handlers guard with their return-count check.

use sb_render::console::{Console, DEFAULT_BG, DEFAULT_FG};

use super::{illegal, out_of_range};
use crate::value::{RuntimeError, SbStr, Value};

/// `LOCATE [x],[y][,z]` — move the text cursor. X/Y are integer cells, Z a float depth;
/// any may be omitted (a [`Value::Void`] placeholder) to keep its previous value.
pub fn locate(
    console: &mut Console,
    args: &[Value],
    wants_value: bool,
) -> Result<(), RuntimeError> {
    if wants_value {
        return Err(illegal()); // used as a function — errnum 4
    }
    if !matches!(args.len(), 2 | 3) {
        return Err(illegal());
    }
    if let Some(x) = opt_int(&args[0])? {
        if !(0..=50).contains(&x) {
            return Err(out_of_range());
        }
        console.cur_x = x as usize;
    }
    if let Some(y) = opt_int(&args[1])? {
        if !(0..=29).contains(&y) {
            return Err(out_of_range());
        }
        console.cur_y = y as usize;
    }
    if args.len() == 3 {
        if let Some(z) = opt_real(&args[2])? {
            // Depth is validated against the documented bounds but not modeled by the
            // 2-D console grid (z-ordering arrives with the compositor, M2).
            if !(-256.0..=1024.0).contains(&z) {
                return Err(out_of_range());
            }
        }
    }
    Ok(())
}

/// `COLOR fg[,bg]` — set the console drawing/background palette indices (0..15). An
/// omitted operand keeps the previous value.
pub fn color(console: &mut Console, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value {
        return Err(illegal());
    }
    if !matches!(args.len(), 1 | 2) {
        return Err(illegal());
    }
    if let Some(fg) = opt_int(&args[0])? {
        console.fg = color_index(fg)?;
    }
    if args.len() == 2 {
        if let Some(bg) = opt_int(&args[1])? {
            console.bg = color_index(bg)?;
        }
    }
    Ok(())
}

/// `CLS` — clear the console grid and home the cursor (keeps the current COLOR). Takes no
/// arguments and returns nothing.
pub fn cls(console: &mut Console, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || !args.is_empty() {
        return Err(illegal());
    }
    console.cls();
    Ok(())
}

/// `ACLS` — reset the draw settings to their start-up state. Accepts 0 args (full reset)
/// or the corpus-verified 3-arg selective form (per-flag meaning oracle-pending, so the
/// 3-arg form performs the same console reset here). Resets the console color/attribute
/// to defaults and clears it; the VM resets its remaining screen state alongside.
pub fn acls(console: &mut Console, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
    if wants_value || !matches!(args.len(), 0 | 3) {
        return Err(illegal());
    }
    console.fg = DEFAULT_FG;
    console.bg = DEFAULT_BG;
    console.set_attr(0);
    console.cls();
    Ok(())
}

/// `INKEY$()` — pop one key from the keyboard buffer without waiting. Headless there is no
/// live buffer, so this always returns the empty string (the documented no-key result).
/// Any argument raises errnum 4.
pub fn inkey(args: &[Value]) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(illegal());
    }
    Ok(Value::Str(SbStr::new()))
}

/// `CHKCHR(x,y)` — read the UTF-16 code of the glyph currently displayed at console cell
/// (x,y). Function only: exactly 2 arguments AND a return value requested, else errnum 4
/// (a wrong arg count or statement use; `chkchr.yaml`, hw_verified). An out-of-bounds
/// coordinate — negative, or at/past the grid edge — returns 0 with **no error**, the same
/// value an empty (cleared) cell reads as.
pub fn chkchr(console: &Console, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
    if !wants_value || args.len() != 2 {
        return Err(illegal());
    }
    let x = args[0].to_int()?;
    let y = args[1].to_int()?;
    if x < 0 || y < 0 {
        return Ok(Value::Int(0));
    }
    // `cell()` already returns a cleared cell (ch == 0) for coordinates past the grid edge,
    // matching the handler's "out-of-bounds returns 0" path.
    Ok(Value::Int(console.cell(x as usize, y as usize).ch as i32))
}

/// Format one `PRINT` item to the UTF-16 code units it puts on the console: a number via
/// the shared [`format_number`](super::format_number) (`%d`/`%g`) contract, a string
/// verbatim. A non-printable type raises Type mismatch (errnum 8), matching the PRINT
/// handler's `else mov r0,#0x8`.
pub fn format_print_item(v: &Value) -> Result<SbStr, RuntimeError> {
    match v {
        Value::Int(_) | Value::Real(_) => Ok(super::format_number(v)?.encode_utf16().collect()),
        Value::Str(s) => Ok(s.clone()),
        _ => Err(super::type_mismatch()),
    }
}

/// Validate a color index is in 0..15, returning it as the palette byte (errnum 10 if out
/// of range).
fn color_index(i: i32) -> Result<u8, RuntimeError> {
    if (0..=15).contains(&i) {
        Ok(i as u8)
    } else {
        Err(out_of_range())
    }
}

/// Read an optional integer operand: [`Value::Void`] (an omitted comma slot) → `None`; a
/// numeric value → `Some(i32)` (a string operand → Type mismatch via [`Value::to_int`]).
fn opt_int(v: &Value) -> Result<Option<i32>, RuntimeError> {
    if matches!(v, Value::Void) {
        Ok(None)
    } else {
        Ok(Some(v.to_int()?))
    }
}

/// Read an optional float operand (see [`opt_int`]).
fn opt_real(v: &Value) -> Result<Option<f64>, RuntimeError> {
    if matches!(v, Value::Void) {
        Ok(None)
    } else {
        Ok(Some(v.to_real()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_sets_cursor() {
        let mut c = Console::top();
        locate(&mut c, &[Value::Int(20), Value::Int(15)], false).unwrap();
        assert_eq!((c.cur_x, c.cur_y), (20, 15));
    }

    #[test]
    fn locate_omitted_keeps_previous() {
        let mut c = Console::top();
        locate(&mut c, &[Value::Int(10), Value::Int(5)], false).unwrap();
        // `LOCATE ,8` keeps X, sets Y.
        locate(&mut c, &[Value::Void, Value::Int(8)], false).unwrap();
        assert_eq!((c.cur_x, c.cur_y), (10, 8));
    }

    #[test]
    fn locate_x_edge_and_out_of_range() {
        let mut c = Console::top();
        // X = 50 is the off-screen edge — accepted.
        locate(&mut c, &[Value::Int(50), Value::Int(0)], false).unwrap();
        // X = 51 is out of range (errnum 10).
        assert_eq!(
            locate(&mut c, &[Value::Int(51), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            10
        );
        // Y = 30 is out of range.
        assert_eq!(
            locate(&mut c, &[Value::Int(0), Value::Int(30)], false)
                .unwrap_err()
                .errnum,
            10
        );
    }

    #[test]
    fn locate_arg_count_and_function_use() {
        let mut c = Console::top();
        // 1 slot → errnum 4.
        assert_eq!(
            locate(&mut c, &[Value::Int(0)], false).unwrap_err().errnum,
            4
        );
        // Used as a function → errnum 4.
        assert_eq!(
            locate(&mut c, &[Value::Int(0), Value::Int(0)], true)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn color_sets_and_checks_range() {
        let mut c = Console::top();
        color(&mut c, &[Value::Int(7), Value::Int(4)], false).unwrap();
        assert_eq!((c.fg, c.bg), (7, 4));
        assert_eq!(
            color(&mut c, &[Value::Int(16)], false).unwrap_err().errnum,
            10
        );
        // `COLOR 0,16`: the fg (0, valid) is applied before the bg range check fails — a
        // partial mutation, matching the disassembled handler (fg stored, then bg checked).
        color(&mut c, &[Value::Int(7), Value::Int(4)], false).unwrap();
        assert_eq!(
            color(&mut c, &[Value::Int(0), Value::Int(16)], false)
                .unwrap_err()
                .errnum,
            10
        );
        assert_eq!(c.fg, 0); // fg was applied
                             // `COLOR ,3` changes only the background, keeping fg.
        color(&mut c, &[Value::Void, Value::Int(3)], false).unwrap();
        assert_eq!((c.fg, c.bg), (0, 3));
    }

    #[test]
    fn cls_clears_keeps_color_and_guards_args() {
        let mut c = Console::top();
        color(&mut c, &[Value::Int(3), Value::Int(4)], false).unwrap();
        c.print_str("X");
        cls(&mut c, &[], false).unwrap();
        assert_eq!(c.cell(0, 0).ch, 0);
        assert_eq!((c.fg, c.bg), (3, 4));
        assert_eq!(cls(&mut c, &[Value::Int(0)], false).unwrap_err().errnum, 4);
        assert_eq!(cls(&mut c, &[], true).unwrap_err().errnum, 4);
    }

    #[test]
    fn acls_resets_and_guards_args() {
        let mut c = Console::top();
        color(&mut c, &[Value::Int(3), Value::Int(4)], false).unwrap();
        acls(&mut c, &[], false).unwrap();
        assert_eq!((c.fg, c.bg), (DEFAULT_FG, DEFAULT_BG));
        // 0 or 3 args ok; 1 or 2 → errnum 4.
        acls(
            &mut c,
            &[Value::Int(1), Value::Int(1), Value::Int(0)],
            false,
        )
        .unwrap();
        assert_eq!(acls(&mut c, &[Value::Int(1)], false).unwrap_err().errnum, 4);
        assert_eq!(
            acls(&mut c, &[Value::Int(1), Value::Int(1)], false)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn chkchr_reads_grid_and_guards() {
        let mut c = Console::top();
        c.locate(0, 0);
        c.print_str("A");
        // Reading the printed cell returns its UTF-16 code (ASC("A") == 65).
        assert_eq!(
            chkchr(&c, &[Value::Int(0), Value::Int(0)], true).unwrap(),
            Value::Int(65)
        );
        // An empty cell reads as 0.
        assert_eq!(
            chkchr(&c, &[Value::Int(10), Value::Int(10)], true).unwrap(),
            Value::Int(0)
        );
        // Out-of-bounds (negative or past the edge) returns 0, no error.
        assert_eq!(
            chkchr(&c, &[Value::Int(-1), Value::Int(0)], true).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            chkchr(&c, &[Value::Int(0), Value::Int(100)], true).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            chkchr(&c, &[Value::Int(60), Value::Int(0)], true).unwrap(),
            Value::Int(0)
        );
        // Wrong arg count → errnum 4.
        assert_eq!(chkchr(&c, &[Value::Int(0)], true).unwrap_err().errnum, 4);
        // Used as a statement (no return requested) → errnum 4.
        assert_eq!(
            chkchr(&c, &[Value::Int(0), Value::Int(0)], false)
                .unwrap_err()
                .errnum,
            4
        );
    }

    #[test]
    fn inkey_empty_and_arg_guard() {
        assert_eq!(inkey(&[]).unwrap(), Value::Str(SbStr::new()));
        assert_eq!(inkey(&[Value::Int(1)]).unwrap_err().errnum, 4);
    }

    #[test]
    fn print_item_formats_numbers_and_strings() {
        assert_eq!(
            format_print_item(&Value::Int(-5)).unwrap(),
            "-5".encode_utf16().collect::<Vec<u16>>()
        );
        assert_eq!(
            format_print_item(&Value::Real(3.0)).unwrap(),
            "3".encode_utf16().collect::<Vec<u16>>()
        );
        assert_eq!(
            format_print_item(&Value::str_from("HI")).unwrap(),
            "HI".encode_utf16().collect::<Vec<u16>>()
        );
    }
}
