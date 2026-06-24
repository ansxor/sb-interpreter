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
//! Number→string formatting splits two ways, both reproduced from the disassembled
//! C `sprintf` calls and confirmed against the oracle:
//!   * [`format_number`] — the **STR$** contract (handler @0x1eb2a8): integers via
//!     `%d`, doubles via `%g` to 6 significant figures (fixed unless the decimal
//!     exponent is `< -4` or `>= 6`, else lowercase exponential with a signed ≥2-digit
//!     exponent; trailing zeros stripped) — STR$(12345678.0)="1.23457e+07".
//!   * [`format_print_number`] — the **PRINT** contract (handler @0x180a50): integers
//!     via `%d`, doubles via `%.8f` (fixed, 8 fractional digits, NEVER exponential)
//!     with trailing zeros and a bare trailing `.` stripped — PRINT 12345678.0 shows
//!     "12345678", PRINT 0.00001 shows "0.00001". Signed zero is preserved by both
//!     (STR$(-0.0)="-0", PRINT -0.0 shows "-0"; sb-oracle 2026-06-23).

pub(crate) mod bg;
pub(crate) mod console;
pub(crate) mod data;
pub(crate) mod device;
pub(crate) mod files;
pub(crate) mod graphics;
pub(crate) mod input;
mod math;
pub(crate) mod prg;
pub(crate) mod screen;
pub(crate) mod sound;
pub(crate) mod sprite;
mod string;

use crate::value::{RuntimeError, SbStr, Value};

// errnums shared across the builtins (names per `spec/reference/errors.yaml`).
pub(crate) const ERR_SYNTAX: u32 = 3;
pub(crate) const ERR_ILLEGAL_FUNCTION_CALL: u32 = 4;
pub(crate) const ERR_TYPE_MISMATCH: u32 = 8;
pub(crate) const ERR_OVERFLOW: u32 = 9;
pub(crate) const ERR_OUT_OF_RANGE: u32 = 10;
const ERR_UNDEFINED_FUNCTION: u32 = 16;
pub(crate) const ERR_SUBSCRIPT_OUT_OF_RANGE: u32 = 31;

/// Build a `Syntax error` (errnum 3) — e.g. the `EFC*` commands' wrong-arg-count gate.
pub(crate) fn syntax_error() -> RuntimeError {
    RuntimeError::new(ERR_SYNTAX)
}
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
/// Build a `Subscript out of range` (errnum 31) — e.g. POP/SHIFT of an empty array.
pub(crate) fn subscript_out_of_range() -> RuntimeError {
    RuntimeError::new(ERR_SUBSCRIPT_OUT_OF_RANGE)
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
    // Console attributes / font (S-T5c; M1-T14 increment).
    "ATTR",
    "SCROLL",
    "WIDTH",
    "FONTDEF",
    // CHKCHR(x,y) reads the VM-owned console grid (function only), so the VM routes it
    // directly like the other console builtins (M1-T14 increment).
    "CHKCHR",
    // Frame-timing stubs (M1-T14): WAIT/VSYNC are no-ops until M4, but registering them
    // lets programs that use them parse and run without auto-declaring them as variables.
    "WAIT",
    "VSYNC",
    // Hardware input (M4-T1): BUTTON reads the button bitmask under a feature ID; STICK/
    // STICKEX read the analog Circle Pad / Circle Pad Pro axes into OUT vars; BREPEAT
    // configures BUTTON feature-1 key-repeat. They read/mutate the VM-owned `InputState`,
    // so the VM routes them directly (like the graphics/sprite/BG commands).
    "BUTTON",
    "STICK",
    "STICKEX",
    "BREPEAT",
    // Touch panel + function keys (M4-T2): TOUCH reads the lower-screen touch state into 3
    // OUT vars; KEY binds/reads a function-key string. Both read/mutate the VM-owned
    // `InputState`, so the VM routes them directly. (INKEY$ stays registered under Console
    // I/O above but now drains the InputState keyboard queue.)
    "TOUCH",
    "KEY",
    // Graphics — GRP page model + color helpers (M2-T1). The page-state commands mutate
    // the VM-owned `GrpState`, so the VM routes them directly (like the console builtins);
    // they are registered here so the compiler treats them as builtins, not variables.
    "GPAGE",
    "GCLS",
    "GCOLOR",
    "GPRIO",
    "GCLIP",
    "RGB",
    "RGBREAD",
    "GSPOIT",
    // Drawing primitives (M2-T2): plot/line/box/fill/circle/triangle/flood-fill write the
    // VM-owned `GrpState` manipulation page, so the VM routes them like the page commands.
    "GPSET",
    "GLINE",
    "GBOX",
    "GFILL",
    "GCIRCLE",
    "GTRI",
    "GPAINT",
    // Bitmap ops (M2-T3): GCOPY blits page→page; GSAVE/GLOAD transfer a page region to/from
    // a numeric array. They mutate the `GrpState` page buffers and (GSAVE) the caller's
    // array, so the VM routes them like the other graphics commands.
    "GCOPY",
    "GSAVE",
    "GLOAD",
    // Sprite lifecycle (M3-T1): SPSET/SPCLR/SPSHOW/SPHIDE create/release/show/hide sprites
    // in the VM-owned `SpriteState`; SPUSED queries a slot. The VM routes them directly
    // like the graphics commands (they mutate/read sprite-system state, not return a pure
    // value), so they are registered here for the compiler to treat as builtins.
    "SPSET",
    "SPCLR",
    "SPSHOW",
    "SPHIDE",
    "SPUSED",
    // Sprite animation / link / vars (M3-T2): SPANIM defines+starts a keyframe animation;
    // SPSTART/SPSTOP resume/pause it; SPFUNC binds a CALL SPRITE callback; SPVAR reads/writes
    // the 8 internal variables; SPLINK/SPUNLINK parent-link sprites (SPLINK also has a
    // function form). All route over the VM-owned `SpriteState`.
    "SPANIM",
    "SPSTART",
    "SPSTOP",
    "SPFUNC",
    "SPVAR",
    "SPLINK",
    "SPUNLINK",
    // Sprite positioning (M3-T3 glue): SPOFS sets/reads a sprite's screen position — the
    // minimum transform needed to place sprites for collision; the rest of the transform
    // setters land in a later increment. SPROT (M7-T2) has a function-form get
    // (`A=SPROT(mgmt)`), so it must be a known builtin to compile as a call, not a variable.
    "SPOFS",
    "SPROT",
    // SPPAGE (M7-T2) selects/reads the global sprite render page; its GET is a 0-arg
    // function form (`P=SPPAGE()`), so it must be a known builtin to compile as a call.
    "SPPAGE",
    // Sprite collision + definition templates (M3-T3): SPCOL/SPCOLVEC configure a sprite's
    // collision rect/mask/velocity; SPHITSP/SPHITRC test for collisions; SPHITINFO reads the
    // result; SPCHK reads the animation-status bitmask; SPDEF manages the definition-template
    // table SPSET copies from. All route over the VM-owned `SpriteState`.
    "SPCOL",
    "SPCOLVEC",
    "SPCHK",
    "SPHITSP",
    "SPHITRC",
    "SPHITINFO",
    "SPDEF",
    // BG core (M3-T4): the background-tilemap commands route over the VM-owned `BgState`.
    // BGSCREEN/BGPAGE size the map + pick the tile graphic page; BGPUT/BGGET/BGFILL/BGCLR
    // read/write tile cells; BGOFS/BGROT/BGSCALE/BGHOME/BGCOLOR/BGCLIP are the per-layer
    // transforms; BGSHOW/BGHIDE toggle visibility. They mutate/read BG state (BGGET/BGPAGE/
    // BGCOLOR/BGOFS/BGROT/BGSCALE/BGHOME also have GET forms), so they are registered here
    // for the compiler to treat as builtins.
    "BGSCREEN",
    "BGPAGE",
    "BGPUT",
    "BGGET",
    "BGFILL",
    "BGCLR",
    "BGOFS",
    "BGROT",
    "BGSCALE",
    "BGCOLOR",
    "BGSHOW",
    "BGHIDE",
    "BGHOME",
    "BGCLIP",
    // BG extras (M3-T5): BGANIM defines+starts a keyframe animation; BGSTART/BGSTOP
    // resume/pause it; BGCHK reads the animation-status bitmask; BGVAR reads/writes the 8
    // internal variables; BGFUNC binds a CALL BG callback; BGCOPY block-copies tilemap
    // cells; BGCOORD converts between BG-screen and display coordinates; BGLOAD/BGSAVE copy
    // tile data to/from a numeric array. All route over the VM-owned `BgState`.
    "BGANIM",
    "BGSTART",
    "BGSTOP",
    "BGCHK",
    "BGVAR",
    "BGFUNC",
    "BGCOPY",
    "BGCOORD",
    "BGLOAD",
    "BGSAVE",
    // Array data-ops (M1-T14): SORT/RSORT mutate their array arguments in place, so the
    // VM routes them to `data::sort` rather than the value-returning `dispatch`.
    "SORT",
    "RSORT",
    // Array block ops (M1-T14): COPY copies array→array or a DATA sequence→array (the
    // DATA form needs the program's DATA pool, so the VM handles it); FILL overwrites
    // elements with a value. Both mutate their destination array (shared by Rc).
    "COPY",
    "FILL",
    // Array stack/queue ops (M1-T14): PUSH/UNSHIFT grow and POP/SHIFT shrink their
    // first operand (a 1D array shared by Rc, or a string variable by reference), so the
    // VM routes them to `data::{push,pop,shift,unshift}` rather than the stateless dispatch.
    "PUSH",
    "POP",
    "SHIFT",
    "UNSHIFT",
    // Screen configuration (M4-T4): XSCREEN sets the screen mode + sprite/BG split; DISPLAY
    // selects/reads the output screen; VISIBLE toggles the four display layers; HARDWARE
    // reports the hardware model. They route over the VM-owned `ScreenConfig` (and HARDWARE
    // is a read-only bare-name sysvar, like PI a zero-arg builtin), so they are registered
    // here for the compiler to treat as builtins rather than variables.
    "XSCREEN",
    "DISPLAY",
    "VISIBLE",
    "HARDWARE",
    // BGM commands (M5-T3): BGMPLAY/BGMSTOP/BGMCHK/BGMVAR/BGMVOL/BGMSET/BGMSETD/BGMCLEAR
    // manage the VM-owned `AudioState` (registered user tunes + per-track transport state).
    // BGMSETD additionally reads MML from the program's DATA pool, so the VM routes it like
    // COPY; the rest route over `AudioState`. They are registered here so the compiler treats
    // them as builtins rather than variables.
    "BGMPLAY",
    "BGMSTOP",
    "BGMCHK",
    "BGMVAR",
    "BGMVOL",
    "BGMSET",
    "BGMSETD",
    "BGMCLEAR",
    // SFX / voice (M5-T4): BEEP plays a preset sound effect; TALK/TALKCHK/TALKSTOP drive
    // synthesized speech; EFCSET/EFCON/EFCOFF/EFCWET control the music effector; WAVSET/
    // WAVSETA define user MML instruments (@224-255). They route over `AudioState`, so they
    // are registered here for the compiler to treat as builtins rather than variables.
    "BEEP",
    "TALK",
    "TALKCHK",
    "TALKSTOP",
    "EFCSET",
    "EFCON",
    "EFCOFF",
    "EFCWET",
    "WAVSET",
    "WAVSETA",
    // File commands (M6-T2): SAVE/LOAD/DELETE/RENAME a resource, FILES lists, CHKFILE tests
    // existence, PROJECT reads/sets the current project. They operate on the VM-owned
    // `Storage` (M6-T1) + current project, so the VM routes them via `call_files` rather than
    // the stateless `dispatch`. (COPY is the array op, registered with the data builtins.)
    "SAVE",
    "LOAD",
    "FILES",
    "DELETE",
    "RENAME",
    "CHKFILE",
    "PROJECT",
    // Source-code manipulation (M6-T4): PRGEDIT selects the edit target (slot + current
    // line); PRGGET$/PRGSET/PRGINS/PRGDEL read/replace/insert/delete the current line;
    // PRGNAME$/PRGSIZE report a slot's file name / line-char-free counts. They read/mutate
    // the VM-owned program-slot source + edit-target state, so the VM routes them via
    // `call_prg` rather than the stateless `dispatch`.
    "PRGEDIT",
    "PRGGET$",
    "PRGSET",
    "PRGINS",
    "PRGDEL",
    "PRGNAME$",
    "PRGSIZE",
    // Faithful limitation stubs (M6-T5): the special-hardware feature gate (XON/XOFF), the
    // microphone (MIC*), the motion sensors (GYRO*/ACCEL), wireless multiplayer (MP*) and the
    // DIALOG modal box. None of the underlying hardware exists in the headless interpreter, so
    // the VM routes them via `call_device` to reproduce their *observable* behavior — the
    // arg-shape / range / type guards and the XON-MIC / XON-MOTION availability errors (36/37)
    // — rather than the device itself. (XON/XOFF are also recognised by the parser's keyword
    // form `XON feature`; registering them here keeps `is_builtin` consistent.)
    "XON",
    "XOFF",
    "MICSTART",
    "MICSTOP",
    "MICDATA",
    "MICSAVE",
    "GYROA",
    "GYROV",
    "GYROSYNC",
    "ACCEL",
    "MPSTART",
    "MPEND",
    "MPSET",
    "MPSTAT",
    "MPSEND",
    "MPRECV",
    "MPGET",
    "MPNAME$",
    "DIALOG",
    // Test-mode assertion (M1-T14): the VM handles it directly (a false condition halts
    // with `VmError::Assert`), so it is not in the stateless `dispatch`.
    "ASSERT__",
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

// -- number formatting (STR$ uses %g; PRINT uses %.8f; FORMAT$ has its own) -----

/// Format a numeric [`Value`] the way **STR$** does: an Integer via C `%d`, a Double
/// via C `%g` at 6 significant figures (disassembled STR$ contract — handler @0x1eb2a8,
/// fmt "%g" @0x1eb4a8). A non-numeric value raises Type mismatch (errnum 8). PRINT uses
/// a different formatter — see [`format_print_number`].
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
/// `1e-05`, `3.14159`). Round-half-to-even and the full magnitude range match C `%g`
/// — verified against a 2000-case bit-exact sweep and the oracle (M7-T4).
pub fn format_g(x: f64, prec: usize) -> String {
    let prec = prec.max(1);
    if x == 0.0 {
        // C `%g` keeps the sign of negative zero: STR$(-0.0)="-0" (sb-oracle 2026-06-23).
        return if x.is_sign_negative() { "-0" } else { "0" }.to_string();
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

/// Format a numeric [`Value`] the way **PRINT** / console output does: an Integer via
/// C `%d`, a Double via [`format_fixed8`]. Distinct from STR$'s `%g` ([`format_number`]).
/// A non-numeric value raises Type mismatch (errnum 8).
pub fn format_print_number(v: &Value) -> Result<String, RuntimeError> {
    match v {
        Value::Int(i) => Ok(i.to_string()),
        Value::Real(d) => Ok(format_fixed8(*d)),
        _ => Err(type_mismatch()),
    }
}

/// SmileBASIC's PRINT double format: C `sprintf("%.8f", x)` then trailing zeros and a
/// bare trailing `.` removed — fixed notation with up to 8 fractional digits, NEVER
/// exponential (disassembled PRINT handler @0x180a50: fmt "%.8f" @0x180b0c via sprintf
/// @0x1e5784, followed by the back-scanning trim loop @0x180a8c). So PRINT 12345678.0
/// shows "12345678", PRINT 0.00001 shows "0.00001", PRINT 0.000000001 shows "0" (rounds
/// below 1e-8), and signed zero is kept (PRINT -0.0 shows "-0"). hw_verified via the
/// sb-oracle console read-back (2026-06-23).
pub fn format_fixed8(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    // `{:.8}` rounds round-half-to-even, matching C `%.8f`; Rust keeps the sign of -0.0.
    trim_fraction(&format!("{x:.8}"))
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
    // The float-format tables use literals near π/e (e.g. 3.14159265, 2.718281828) as
    // deliberate STR$/PRINT inputs, not as constants — silence the approx-constant lint.
    #![allow(clippy::approx_constant)]
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
        assert_eq!(format_g(-0.0, 6), "-0"); // signed zero kept (hw_verified)
        assert_eq!(format_g(std::f64::consts::TAU, 6), "6.28319");
    }

    #[test]
    fn format_g_str_dollar_table() {
        // STR$ doubles = C `%g` at 6 sig figs (handler @0x1eb2a8). Values cross-checked
        // against a 2000-case bit-exact Python `%.6g` sweep + the oracle (2026-06-23).
        let cases: &[(f64, &str)] = &[
            (100000.0, "100000"),
            (1000000.0, "1e+06"), // exponent threshold exp>=6
            (999999.0, "999999"),
            (999999.5, "1e+06"), // round-up carries the exponent
            (0.0001, "0.0001"),
            (0.00001, "1e-05"), // exponent threshold exp<-4
            (1234567.0, "1.23457e+06"),
            (0.000123456, "0.000123456"),
            (12345678.0, "1.23457e+07"),
            (16777216.0, "1.67772e+07"),
            (1e20, "1e+20"),
            (1e-20, "1e-20"),
            (1e100, "1e+100"), // 3-digit exponent
            (1e-300, "1e-300"),
            (-1e-300, "-1e-300"),
            (9.999995, "10"), // round-half-to-even carry
            (99.99996, "100"),
            (1234.5678, "1234.57"),
            (-0.0000012345, "-1.2345e-06"),
            (0.9999999, "1"),
            (2.718281828, "2.71828"),
            (0.123456785, "0.123457"),
            (0.000000001, "1e-09"),
            (0.0, "0"),
            (-0.0, "-0"),
        ];
        for (v, want) in cases {
            assert_eq!(format_g(*v, 6), *want, "STR$ format of {v}");
        }
    }

    #[test]
    fn format_print_fixed8_table() {
        // PRINT doubles = C `%.8f` + trailing-zero/dot trim (handler @0x180a50). All
        // values hw_verified via the sb-oracle console read-back (2026-06-23).
        let cases: &[(f64, &str)] = &[
            (12345678.0, "12345678"),   // never exponential (STR$ -> "1.23457e+07")
            (3.14159265, "3.14159265"), // 8 fractional digits
            (0.00001, "0.00001"),       // never exponential (STR$ -> "1e-05")
            (1.0 / 3.0, "0.33333333"),
            (-3.14159265, "-3.14159265"),
            (0.5, "0.5"),
            (180.0, "180"),
            (0.000000001, "0"),         // 1e-9 rounds below 1e-8 -> 0
            (0.00000001, "0.00000001"), // 1e-8 = exactly the 8th decimal
            (100000000000.0, "100000000000"),
            (0.123456785, "0.12345678"), // round-half-to-even keeps the even 8
            (2.718281828, "2.71828183"),
            (-0.0, "-0"), // signed zero preserved
            (2.0_f64.sqrt(), "1.41421356"),
        ];
        for (v, want) in cases {
            assert_eq!(format_fixed8(*v), *want, "PRINT format of {v}");
        }
        assert_eq!(format_print_number(&Value::Int(-5)).unwrap(), "-5");
        assert_eq!(
            format_print_number(&sb_string("x")).unwrap_err().errnum,
            ERR_TYPE_MISMATCH
        );
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
        assert!(b.is_builtin("SPSET")); // M3-T1
        assert!(b.is_builtin("SPOFS")); // M3-T3 (positioning glue for collision)
        assert!(b.is_builtin("SPHITSP")); // M3-T3
        assert!(b.is_builtin("SPDEF")); // M3-T3
        assert!(b.is_builtin("SPROT")); // M7-T2 (function-form get A=SPROT(mgmt))
    }
}
