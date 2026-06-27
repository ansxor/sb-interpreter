//! Stack VM (M1-T6) — runs the [`Program`] the compiler (M1-T5) emits.
//!
//! Per `spec/concepts/execution-model.md` and `prd/M1.md`, this is a stack machine:
//! an operand stack of [`Value`]s, a program counter `pc`, a `GOSUB` return-address
//! stack, and a stack of `DEF`-call frames (each with its own bp-relative locals).
//! Opcodes are dispatched by a `match` over the flat [`Op`] enum (chosen over osb's
//! object-per-opcode `Code` for Rust/wasm + determinism; the *semantics* mirror
//! `VM.d`'s `run()`). The crate stays I/O-free, so the VM builds for `wasm32`.
//!
//! ## What this slice runs
//!
//! The language core: constants, scalar variables, 1–4D arrays (new/read/write),
//! the full operator set, `IF`/`FOR`/`WHILE`/`REPEAT` (lowered to jumps by the
//! compiler), `GOTO`/`GOSUB`/`RETURN`, `ON … GOTO/GOSUB`, `DEF`/`COMMON DEF` calls
//! with by-value params, `OUT` results and a return value, `DATA`/`READ`/`RESTORE`,
//! and scalar `INC`/`DEC`/`SWAP`. Recursion past [`DEF_STACK_LIMIT`] (DEF call frames)
//! or [`GOSUB_STACK_LIMIT`] (GOSUB return addresses) raises **Stack overflow** (errnum 5).
//!
//! Builtins ([`Op::CallBuiltin`]/[`Op::CallDynamic`], M1-T7) and console + input
//! (`Print*`/`Input`/`Linput` plus the `LOCATE`/`COLOR`/`BACKCOLOR`/`CLS`/`ACLS`/`INKEY$`
//! builtins, M1-T8) are wired: the VM owns an [`sb_render::console::Console`] that `PRINT`
//! and the console commands drive, and a headless input queue ([`Vm::push_input`]) feeds
//! `INPUT`/`LINPUT`. Array-element references ([`Op::PushArrayRef`]) are wired (`SWAP`/
//! `INC`/`DEC`/`OUT` on `A[i]`); `USE`/`EXEC` (M6-T6) validate their operands with the
//! hw_verified slot/resource error model, `USE` marks a slot executable (loading its program
//! so `COMMON DEF`s resolve cross-slot), and `EXEC` performs the multi-program control
//! transfer (load-from-storage, slot switch, `END`-returns-to-launcher). Runtime-name
//! references ([`Op::PushRefExpr`]/[`Op::PopRef`], `VAR()`) are not yet wired and raise
//! [`VmError::Unsupported`] rather than panicking — their handler lands in the milestones
//! above.
//!
//! ## Operator semantics (from the spec/disassembly)
//!
//! `+`/`-`/`*` (M7-T5, hw_verified sb-oracle 2026-06-24) are **type-aware**: when both
//! operands are statically Integer (`%` var, integer literal, bit-op result) the op WRAPS
//! mod 2³² (`A%=2000000000:A%*2 → -294967296`), but when an operand is Real/Number-typed
//! (suffix-less / `#` / Real literal / `/`-result / constant fold) overflow PROMOTES the
//! result Int→Double (`A=2147483647:A+1 → 2147483648.0`) — the compiler picks the path via
//! `is_real_typed` and emits [`Op::Operate`] (wrap, [`operate`]) or [`Op::OperatePromote`]
//! ([`operate_promote`]). Storing a Double outside `[i32::MIN, i32::MAX]` into a `%` target
//! → Overflow (errnum 9, [`Value::to_int_store`]). `/` is **always** real division
//! (`7/2 == 3.5`), divide-by-zero → errnum 7. `DIV`/`MOD`/`AND`/`OR`/`XOR` **truncate each
//! operand toward zero to `i32` first** (`7 AND 2.9 == 2`) then do the (wrapping) integer
//! op; `DIV`/`MOD` by a (truncated) zero → errnum 7. The shifts `<<`/`>>` also truncate
//! operands but the COUNT is not masked mod 32 (left `≥32` or `<0` → 0; right is arithmetic,
//! clamped to 31) — see [`shift`]. Comparisons yield Integer `1`/`0`; strings compare by
//! UTF-16 code units; a string-vs-number compare or any string in an arithmetic op → Type
//! mismatch (errnum 8). (`spec/instructions/{div,mod,and,or,xor}.yaml` +
//! `harness/corpus/cases/overflow.yaml`, hw_verified.)

use crate::array::SbArray;
use crate::ast::{BinOp, Name, UnOp};
use crate::builtins::screen::ScreenConfig;
use crate::builtins::sound::AudioState;
use crate::bytecode::{CallbackKind, Const, Op, Program, VarRef, VarType};
use crate::clock::{FrameClock, WallClock};
use crate::input::InputState;
use crate::input_buffer::InputBuffer;
use crate::storage::{
    parse_files_filter, parse_resource, FilesFilter, Folder, MemStorage, ResourceKind, Storage,
    DEFAULT_PROJECT,
};
use crate::sysvars::Sysvar;
use crate::token::Suffix;
use crate::value::{Cell, ElemRef, RuntimeError, SbStr, Value};
use sb_render::bg::BgState;
use sb_render::console::Console;
use sb_render::grp::{GrpState, GRP_SCREEN_COUNT};
use sb_render::sprite::{SpdefTable, SpriteState};
use std::cmp::Ordering;

/// Max depth of the `DEF` call-frame stack before raising **Stack overflow** (errnum 5).
/// hw_verified 2026-06-26 (sb-oracle binary search, `harness/harvest/out/stackdepth.tsv`):
/// real SB 3.6.0 completes a DEF recursion of 2730 frames and overflows when the 2731st is
/// pushed (`DEF F:N=N+1:IF N<D THEN F`: D=2730 runs clean, D=2731 trips). sb-core checks
/// `frames.len() >= DEF_STACK_LIMIT` *before* pushing, so `2730` trips the 2731st push.
pub const DEF_STACK_LIMIT: usize = 2730;

/// Max depth of the `GOSUB` return-address stack before raising **Stack overflow** (errnum 5).
/// hw_verified 2026-06-26 (sb-oracle binary search, `harness/harvest/out/stackdepth_gosub.tsv`):
/// real SB 3.6.0 completes a GOSUB nesting of 16384 frames and overflows when the 16385th is
/// pushed (`N=0:GOSUB @A:…:@A:N=N+1:IF N<D THEN GOSUB @A:RETURN`: D=16384 clean, D=16385
/// trips). The GOSUB limit is ~6× the DEF limit, so real SB keeps **separate** stacks for
/// GOSUB return addresses vs DEF call frames — not one combined limit. sb-core checks
/// `gosub.len() >= GOSUB_STACK_LIMIT` before pushing, so `16384` trips the 16385th push.
/// (The mixed GOSUB+DEF trip depth is unprobed; the two limits are applied independently —
/// `bd:sb-interpreter-air`.)
pub const GOSUB_STACK_LIMIT: usize = 16384;

/// The `FREEMEM` system variable's reported free user memory (M6-T3). SmileBASIC computes this
/// from its real allocator, so it *decreases* as a program DIMs arrays / defines resources;
/// `sb-core` does not model the allocator, so it reports a fixed faithful constant. The value
/// is anchored to real SB 3.6.0: a near-empty program reported `8314876` (sb-oracle 2026-06-23).
/// Programs that branch on FREEMEM (low-memory guards) therefore see "plenty free"; modelling
/// the allocator so FREEMEM tracks real usage is queued (`bd:sb-interpreter-air`).
const DEFAULT_FREEMEM: i32 = 8_314_876;

// errnums used directly by the VM (names per `spec/reference/errors.yaml`).
const ERR_ILLEGAL_FUNCTION_CALL: u32 = 4;
const ERR_STACK_OVERFLOW: u32 = 5;
const ERR_STACK_UNDERFLOW: u32 = 6;
const ERR_DIVIDE_BY_ZERO: u32 = 7;
const ERR_TYPE_MISMATCH: u32 = 8;
const ERR_OUT_OF_RANGE: u32 = 10;
const ERR_OUT_OF_DATA: u32 = 13;
const ERR_UNDEFINED_LABEL: u32 = 14;
const ERR_UNDEFINED_FUNCTION: u32 = 16;
const ERR_RETURN_WITHOUT_GOSUB: u32 = 30;
const ERR_SUBSCRIPT: u32 = 31;
const ERR_USE_PRGEDIT: u32 = 38;
const ERR_LOAD_FAILED: u32 = 46;
/// "Uninitialized variable used" — a Class-1 statement keyword used as a sole
/// bareword expression operand (`A=STOP`) routes to a read of an uninitialized
/// name, raising this at runtime. hw_verified (sb-oracle 2026-06-26,
/// harness/harvest/out/ctm_bareword_kw.tsv). Under OPTION STRICT the same form
/// raises errnum 15 at compile time instead (see `compiler.rs`).
const ERR_UNINITIALIZED_VARIABLE: u32 = 48;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// `END`, or fell off the end of the code.
    End,
    /// `STOP` (distinct from `END`; resumable via `CONT` in DIRECT mode — M1-T13).
    Stop,
}

/// A run failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// A SmileBASIC runtime error: an `ERRNUM` and the 1-based source `line`
    /// (`ERRLINE`) it occurred on.
    Sb { errnum: u32, line: u32 },
    /// An opcode whose handler is implemented in a later milestone (builtins M1-T7,
    /// console/input M1-T8, `USE`/`EXEC` M6, array-element/runtime-name refs).
    Unsupported(&'static str),
    /// A failed `ASSERT__` (the test-mode builtin, M1-T14): the condition was false.
    /// Carries the assertion's message and the 1-based source `line` it fired on. This
    /// is a harness construct, NOT a SmileBASIC runtime error — it has no `ERRNUM`.
    Assert { message: String, line: u32 },
}

impl VmError {
    /// The `ERRNUM` if this is a SmileBASIC runtime error.
    pub fn errnum(&self) -> Option<u32> {
        match self {
            VmError::Sb { errnum, .. } => Some(*errnum),
            VmError::Unsupported(_) | VmError::Assert { .. } => None,
        }
    }

    /// The `ERRLINE` (source line) if this is a SmileBASIC runtime error.
    pub fn errline(&self) -> Option<u32> {
        match self {
            VmError::Sb { line, .. } => Some(*line),
            VmError::Unsupported(_) | VmError::Assert { .. } => None,
        }
    }

    /// The byte-for-byte message SmileBASIC displays for this error (the binary's error-pool
    /// string for this `errnum`; see [`crate::error`]). `None` for the non-SB variants.
    pub fn message(&self) -> Option<&'static str> {
        self.errnum().map(crate::error::error_message)
    }
}

/// One `DEF`/`COMMON DEF` activation record.
#[derive(Clone)]
struct Frame {
    /// Frame-local storage cells, indexed by [`VarRef::Local`]: params, then `OUT`
    /// params, then body-declared/auto-declared locals.
    locals: Vec<Cell>,
    /// `pc` to resume at in the caller after the call returns.
    return_pc: usize,
    /// Index into [`Program::functions`] of the function running in this frame.
    func: usize,
    /// Whether the caller wants the function's return value left on the stack.
    wants_value: bool,
    /// For a cross-slot `COMMON DEF` call (M6-T6): the slot the call came *from*, to
    /// switch the active program/globals back to on return. `None` for a same-slot call.
    caller_slot: Option<u8>,
}

/// A compiled program loaded into a non-running program SLOT (M6-T6), with its own
/// globals storage. While a slot is the *active* (executing) context its program lives
/// in [`Vm::program`]/[`Vm::globals`] and its slot entry is `None`; a cross-slot
/// `COMMON DEF` call swaps it in.
struct LoadedSlot {
    program: Program,
    globals: Vec<Cell>,
}

/// A saved caller execution context for the documented `EXEC` cross-slot return rule
/// (M6-T6). `EXEC`-ing into a *different* slot transfers control there, but the docs say
/// "It is possible to return by using END in a program started with EXEC in another SLOT"
/// — so a program run by a cross-slot `EXEC` returns to its launcher when it reaches `END`
/// (or falls off the end of its code). This record holds the launcher's full resume state,
/// captured the moment control is handed to the target slot. The stack of these supports
/// nesting (A `EXEC`s B `EXEC`s C → C's `END` returns to B, B's to A). A running-slot
/// `EXEC` (restart / `EXEC "PRG0:…"`) is not cross-slot and pushes nothing.
struct ExecReturn {
    /// The launcher's active program SLOT, to swap back to on return.
    slot: u8,
    /// `pc` to resume at in the launcher (the op after the `EXEC` statement).
    pc: usize,
    /// The launcher's operand stack (balanced at the statement boundary, but captured
    /// faithfully in case `EXEC` runs from inside an expression-y context).
    stack: Vec<Value>,
    /// The launcher's `DEF`-call frames (so an `EXEC` from inside a function returns into it).
    frames: Vec<Frame>,
    /// The launcher's `GOSUB` return stack.
    gosub: Vec<usize>,
    /// The launcher's `DATA` read cursor.
    data_cursor: usize,
}

/// Screen-fader state (M4/M5 frame effect) behind `FADE`/`FADECHK`.
///
/// The fader draws in front of the composed screen, blended by its alpha channel. A color with
/// zero alpha disables the fader. A timed `FADE color,time` interpolates linearly from the
/// current color to the target over `time` 1/60-second frames; the interpolation ticks in both
/// the host-driven (`tick_frame`) and headless (`WAIT`/`VSYNC`) frame-advance paths.
#[derive(Debug, Clone, Copy, Default)]
struct Fader {
    /// Color at the start of the current timed fade.
    start: i32,
    /// Target color of the current timed fade.
    target: i32,
    /// Current interpolated color returned by `FADE()`.
    current: i32,
    /// Frames remaining until the timed fade completes.
    remaining: u64,
    /// Total duration of the current timed fade.
    total: u64,
    /// Whether the fader is enabled for rendering (i.e. current alpha is non-zero).
    enabled: bool,
}

impl Fader {
    fn new() -> Self {
        Self {
            start: 0,
            target: 0,
            current: 0,
            remaining: 0,
            total: 0,
            enabled: false,
        }
    }

    /// `FADE color` — instant set, zero duration.
    fn set(&mut self, color: i32) {
        self.start = color;
        self.target = color;
        self.current = color;
        self.remaining = 0;
        self.total = 0;
        self.enabled = ((color as u32) >> 24) != 0;
    }

    /// `FADE color,time` — animate from `current` to `color` over `time` frames.
    fn fade_to(&mut self, color: i32, time: u64) {
        if time == 0 {
            self.set(color);
            return;
        }
        self.start = self.current;
        self.target = color;
        self.total = time;
        self.remaining = time;
        self.enabled = true;
        // At frame 0 current stays at start.
    }

    fn current(&self) -> i32 {
        self.current
    }

    fn is_animating(&self) -> bool {
        self.remaining > 0
    }

    /// Advance the fader by `frames` displayed frames.
    fn tick(&mut self, frames: u64) {
        if self.remaining == 0 {
            return;
        }
        self.remaining = self.remaining.saturating_sub(frames);
        self.update_current();
        if self.remaining == 0 {
            self.enabled = ((self.current as u32) >> 24) != 0;
        }
    }

    fn update_current(&mut self) {
        if self.total == 0 {
            self.current = self.target;
            return;
        }
        let done = (self.total - self.remaining) as u32;
        let total = self.total as u32;
        let lerp = |a: u32, b: u32, sh: u32| -> u32 {
            let av = (a >> sh) & 0xFF;
            let bv = (b >> sh) & 0xFF;
            (av * (total - done) + bv * done + total / 2) / total
        };
        let a = self.start as u32;
        let b = self.target as u32;
        let argb =
            (lerp(a, b, 24) << 24) | (lerp(a, b, 16) << 16) | (lerp(a, b, 8) << 8) | lerp(a, b, 0);
        self.current = argb as i32;
    }
}

/// The stack VM.
pub struct Vm {
    program: Program,
    /// One storage [`Cell`] per [`Program::globals`] entry.
    globals: Vec<Cell>,
    /// The operand stack.
    stack: Vec<Value>,
    /// `DEF`-call activation records.
    frames: Vec<Frame>,
    /// `GOSUB` return addresses (indices into `program.code`).
    gosub: Vec<usize>,
    /// Program counter (index into `program.code`).
    pc: usize,
    /// `DATA` read cursor (index into `program.data`).
    data_cursor: usize,
    /// The 8 TinyMT32 random series behind `RND`/`RNDF`/`RANDOMIZE` (M1-T9).
    rng: crate::rng::Rng,
    /// The text console model (M1-T10): one console per physical DISPLAY screen. Each screen
    /// owns its own grid + cursor + COLOR/ATTR state; `PRINT`/`LOCATE`/etc. route to the active
    /// screen's console (#86).
    consoles: [Console; 2],
    /// The GRP graphics state (M2-T1): 6 pages + page selection / draw color / Z priority
    /// / clip rectangles, driven by `GPAGE`/`GCLS`/`GCOLOR`/`GPRIO`/`GCLIP`/`GSPOIT`. The
    /// compositor (M2-T4) turns the display page into the framebuffer.
    grp: GrpState,
    /// The per-DISPLAY-screen sprite system state (M3-T1): two independent 512-slot sprite
    /// tables (index 0 = Upper, 1 = Touch), driven by the lifecycle commands
    /// `SPSET`/`SPCLR`/`SPSHOW`/`SPHIDE`/`SPUSED`. Every sprite builtin routes to the ACTIVE
    /// screen ([`Vm::active_screen`] = `self.screen.display`), so `DISPLAY 1` sprites are
    /// independent of `DISPLAY 0` sprites (#83). The compositor (M3-T6) draws the live sprites
    /// of a given screen into its framebuffer; the transform/animation setters extend it
    /// (M3-T2+).
    sprites: [SpriteState; GRP_SCREEN_COUNT],
    /// The shared `SPDEF` definition-template table (M3-T3). A single GLOBAL table — `SPDEF`
    /// edits are visible from every DISPLAY screen, so it is owned at VM level (not duplicated
    /// per screen) and threaded into the per-screen [`SpriteState`] template-reading operations.
    spdef: SpdefTable,
    /// The per-DISPLAY-screen BG (background tilemap) system state (M3-T4): two independent
    /// 4-layer tilemap tables (index 0 = Upper, 1 = Touch), each with its own graphic page,
    /// driven by `BGSCREEN`/`BGPUT`/`BGGET`/`BGFILL`/`BGCLR` and the per-layer transforms
    /// `BGOFS`/`BGROT`/`BGSCALE`/`BGHOME`/`BGCOLOR`/`BGSHOW`/`BGHIDE`/`BGCLIP`/`BGPAGE`. Every
    /// BG builtin routes to the ACTIVE screen ([`Vm::active_screen`] = `self.screen.display`),
    /// so `DISPLAY 1` BG layers are independent of `DISPLAY 0` (#84). Unlike sprites there is no
    /// shared/global sub-table — `BGPAGE` is per-screen (like `SPPAGE`). The compositor (M3-T6)
    /// draws a given screen's visible layers into its framebuffer; animation/coord/load-save
    /// (M3-T5) extends it.
    bg: [BgState; GRP_SCREEN_COUNT],
    /// The hardware-input snapshot (M4-T1): the per-frame button masks + analog stick axes
    /// read by `BUTTON`/`STICK`/`STICKEX`, plus the `BREPEAT` key-repeat config. Headless it
    /// is centred/released; the platform layer (M4-T5) fills it each frame and tests drive a
    /// scripted timeline via [`InputState::advance_frame`].
    input: InputState,
    /// The screen background color code (`BACKCOLOR`). The handler round-trips the user's
    /// RGB code, so we store it verbatim; the rendered border color is screen state (M2).
    back_color: i32,
    /// `TABSTEP` — the `PRINT ,` tab-stop width. Boot default 4 (`sysvars.yaml`); writable via
    /// the `TABSTEP = n` system-variable assignment (M6-T3).
    tabstep: usize,
    /// `SYSBEEP` — the system-beep enable flag (M6-T3). Boot default 1 (TRUE = allowed,
    /// `sysvars.yaml`); writable via `SYSBEEP = n`. Stored verbatim so a read round-trips the
    /// written value; the audible UI beep it gates is platform UI (no deterministic golden).
    sysbeep: i32,
    /// `RESULT` — the last DIALOG result (M6-T3). Boot default 1 (TRUE) on real SB 3.6.0 before
    /// any dialog (sb-oracle 2026-06-23); DIALOG (M6-T5) sets it to TRUE/FALSE/-1 (Suspended).
    result: i32,
    /// `CALLIDX` — the index passed into the current SPFUNC/BGFUNC callback (M6-T3). 0 outside
    /// any callback (the hw_verified golden); a `CALL SPRITE`/`CALL BG` sweep (M6-T6) sets it to
    /// each dispatched sprite/layer index and leaves it at the final value afterwards.
    callidx: i32,
    /// Whether a `CALL SPRITE`/`CALL BG` callback sweep is in progress (M6-T6). Armed by
    /// [`Op::CallbackInit`]; cleared when [`Op::CallbackNext`] runs the index past the table.
    callback_active: bool,
    /// `FREEMEM` — free user memory in KB (M6-T3). A fixed faithful model (we don't track the
    /// real allocator); the exact boot value is oracle-pending (`bd:sb-interpreter-air`).
    freemem: i32,
    /// The wall-clock date/time behind `DATE$`/`TIME$` (M6-T3). Deterministic default epoch;
    /// the native host injects the real RTC via [`Vm::set_wall_clock`].
    wall_clock: WallClock,
    /// Line-based input buffer for `INPUT`/`LINPUT`. Headless runners preload completed
    /// lines via [`Vm::push_input`]; interactive hosts type into the current line and
    /// commit with [`Vm::input_enter`].
    input_buffer: InputBuffer,
    /// Error state (M1-T13): the `ERRNUM`/`ERRLINE`/`ERRPRG` read-only sysvars. Boot/`RUN`
    /// reset to 0 (= "No Error"); set at the moment of a halting error and left readable
    /// afterwards (the DIRECT-mode residue — see `spec/concepts/error-model.md`). `ERRPRG`
    /// is the executing program SLOT, always 0 in single-slot M1 (multi-slot → M6).
    errnum: i32,
    errline: i32,
    errprg: i32,
    /// The 60 fps frame clock (M4-T3) behind `MAINCNT`/`VSYNC`/`WAIT`. Headless, it only
    /// advances when a program blocks on frames (VSYNC/WAIT) or the platform ticks it; the
    /// native host paces it to wall-clock 60 fps. See [`crate::clock`].
    clock: FrameClock,
    /// Screen configuration (M4-T4): the `XSCREEN` mode, the `DISPLAY` output target, the
    /// per-screen `VISIBLE` layer flags and the `HARDWARE` model. The compositor reads the
    /// Upper-screen visibility flags via [`Vm::screen_visibility`].
    screen: ScreenConfig,
    /// The screen fader (M4/M5 frame effect) behind `FADE`/`FADECHK`. Drawn in front of the
    /// composed frame; disabled when its alpha byte is zero.
    fader: Fader,
    /// The BGM state (M5-T3): registered user-defined tunes (128..255 → compiled MML) +
    /// per-track transport state, driven by `BGMPLAY`/`BGMSTOP`/`BGMCHK`/`BGMVAR`/`BGMVOL`/
    /// `BGMSET`/`BGMSETD`/`BGMCLEAR`. The live audio backend (M5-T5) renders the playing
    /// tracks through the synth; the audible output has no deterministic golden (O-T7).
    audio: AudioState,
    /// The file/project store (M6-T1/T2) behind `SAVE`/`LOAD`/`FILES`/`DELETE`/`RENAME`/
    /// `CHKFILE`/`PROJECT`. Defaults to an in-memory [`MemStorage`] (wasm-safe, I/O-free); the
    /// native host can swap in a real filesystem impl via [`Vm::set_storage`].
    storage: Box<dyn Storage>,
    /// The current project name (M6-T2). `SAVE`/`LOAD`/`FILES`/… are keyed against it;
    /// `PROJECT OUT name$` reads it. Defaults to [`DEFAULT_PROJECT`].
    current_project: String,
    /// The running program slot (0..3). A bare resource name (`SAVE "NAME"`) targets this
    /// slot's `TXT` namespace. Single-slot in M6-T2 (always 0); multi-slot is M6-T6.
    current_slot: u8,
    /// The four program SLOTs' editable source (M6-T4), edited by the `PRG*` family. Slot 0
    /// is the running program; a host/test can seed any slot via [`Vm::set_slot_source`].
    prg_slots: [crate::builtins::prg::PrgSlot; 4],
    /// Which program SLOTs are marked *executable* by `USE` (M6-T6). Slot 0 (the running
    /// program) is inherently usable; `USE 1`/`USE 2`/`USE 3` mark the others so a
    /// cross-slot `CALL "name"` can resolve their `COMMON DEF`s.
    slot_used: [bool; 4],
    /// Compiled programs loaded into the non-running program SLOTs (M6-T6). A host/test
    /// loads a slot's executable via [`Vm::load_slot_program`]; the *active* slot's entry
    /// is `None` (its program is in [`Vm::program`]/[`Vm::globals`]). A `USE`'d slot's
    /// `COMMON DEF`s become callable from another slot via `CALL "name"`.
    slot_programs: [Option<LoadedSlot>; 4],
    /// The program SLOT whose code is currently executing (M6-T6). 0 is the running
    /// program; a cross-slot `COMMON DEF` call temporarily makes the target slot active
    /// (its program/globals swapped into [`Vm::program`]/[`Vm::globals`]) and restores it
    /// on return. Distinct from [`Vm::current_slot`] (the file-resource default slot).
    active_slot: u8,
    /// Pending cross-slot `EXEC` return contexts (M6-T6). A cross-slot `EXEC` pushes the
    /// launcher's resume state here; when the EXEC'd program reaches `END` (or runs off the
    /// end of its code) the top context is restored, returning control to the launcher — the
    /// documented "return by using END in a program started with EXEC in another SLOT" rule.
    exec_returns: Vec<ExecReturn>,
    /// The active `PRGEDIT` edit target: `(slot, current line index)`. `None` is the *cold*
    /// state — no PRGEDIT has run, so `PRGGET$`/`PRGSET`/`PRGINS`/`PRGDEL` raise errnum 38.
    /// In real SB this state is session-persistent (a shared global, see the `prgset` spec).
    prg_edit: Option<(u8, usize)>,
    /// The special-hardware feature enable flags (M6-T5), toggled by `XON`/`XOFF`. Until the
    /// matching feature is enabled, the microphone / motion instructions raise errnum 36 / 37.
    /// All boot disabled — a fresh program has declared no special feature.
    device: crate::builtins::device::DeviceState,
    /// Set by frame-timing builtins (`VSYNC`/`WAIT`) to request that the host yield control
    /// so the platform can advance a real frame and refresh live input. Used by the wasm
    /// canvas host (and future native frame-yielding host) to avoid blocking the UI thread
    /// inside `BUTTON`-polling loops. Cleared by `run_frame` when it yields.
    yielded: bool,
    /// True once `run_frame` has been called; switches `call_timing` into the host-driven
    /// VBlank model (begin_wait/begin_vsync instead of instant-jump wait/vsync). Never set
    /// by `run()`, so the headless runner and all existing tests keep the instant model.
    interactive: bool,
    /// True while an interactive `INPUT`/`LINPUT` is waiting for the user to press Enter.
    /// The wasm host pauses `run_frame` and shows the input overlay while this is set.
    input_wait: bool,
    /// Set once the prompt/`?` for the current interactive `INPUT`/`LINPUT` has been printed,
    /// so rewinding `pc` to retry the opcode does not re-print it every frame.
    input_prompt_printed: bool,
}

impl Vm {
    /// Build a VM for a compiled program, with every global initialised to its
    /// declared type's zero value (numeric → 0, string → "").
    pub fn new(program: Program) -> Self {
        let defint = program.options.defint;
        let globals = program
            .globals
            .iter()
            .map(|v| Value::cell(Value::default_for_suffix(v.name.suffix, defint)))
            .collect();
        Vm {
            program,
            globals,
            stack: Vec::new(),
            frames: Vec::new(),
            gosub: Vec::new(),
            pc: 0,
            data_cursor: 0,
            rng: crate::rng::Rng::new(),
            consoles: [Console::top(), Console::bottom()],
            grp: GrpState::with_defaults(),
            sprites: std::array::from_fn(|_| SpriteState::new()),
            spdef: SpdefTable::new(),
            bg: std::array::from_fn(|_| BgState::new()),
            input: InputState::new(),
            back_color: 0,
            tabstep: 4,
            sysbeep: 1,
            result: 1,
            callidx: 0,
            callback_active: false,
            freemem: DEFAULT_FREEMEM,
            wall_clock: WallClock::EPOCH,
            input_buffer: InputBuffer::new(),
            input_wait: false,
            input_prompt_printed: false,
            errnum: 0,
            errline: 0,
            errprg: 0,
            clock: FrameClock::new(),
            screen: ScreenConfig::new(),
            fader: Fader::new(),
            audio: AudioState::new(),
            storage: Box::new(MemStorage::new()),
            current_project: DEFAULT_PROJECT.to_string(),
            current_slot: 0,
            prg_slots: Default::default(),
            slot_used: [true, false, false, false],
            slot_programs: Default::default(),
            active_slot: 0,
            exec_returns: Vec::new(),
            prg_edit: None,
            device: Default::default(),
            yielded: false,
            interactive: false,
        }
    }

    /// Seed a program SLOT's source + file name (M6-T4). A host/test loads a slot here so the
    /// `PRG*` family can read/edit it; `slot` is clamped to 0..3, out-of-range is ignored.
    pub fn set_slot_source(&mut self, slot: u8, name: &str, source: &str) {
        if let Some(s) = self.prg_slots.get_mut(slot as usize) {
            s.name = name.encode_utf16().collect();
            s.set_source(&source.encode_utf16().collect::<Vec<u16>>());
        }
    }

    /// Load a compiled program into a non-running program SLOT (M6-T6) so its
    /// `COMMON DEF`s become callable cross-slot once the slot is `USE`'d. `slot` must be
    /// 1..3 (slot 0 is the running program, which lives in the VM directly); other values
    /// are ignored. The slot's globals are initialised to their declared-type zero, like
    /// [`Vm::new`]. A host/test loads slots here; the in-program loader (`LOAD"PRGn:"`/
    /// `EXEC`) that would fill them from storage is the deferred control-transfer model.
    pub fn load_slot_program(&mut self, slot: u8, program: Program) {
        if slot == 0 || slot as usize >= self.slot_programs.len() {
            return;
        }
        self.stash_slot_program(slot, program);
    }

    /// Store a freshly-compiled program into a NON-RUNNING slot's registry entry (M6-T6),
    /// initialising its globals to declared-type zeros. Unlike the host-seeding
    /// [`Vm::load_slot_program`] (slots 1..3) this also accepts slot 0: when slot 0 is not
    /// the running slot its parked program lives in `slot_programs[0]`, so an in-VM
    /// `EXEC`/`USE "PRG0:file"` load into a non-running slot 0 is uniform with the other
    /// slots (osb `VM.d` keeps all program SLOTs in one `slots[]` array — slot 0 is not
    /// special). The caller MUST guarantee `slot != current_slot` (the running program lives
    /// in [`Vm::program`], not the registry, so its entry is `None` while active).
    fn stash_slot_program(&mut self, slot: u8, program: Program) {
        let defint = program.options.defint;
        let globals = program
            .globals
            .iter()
            .map(|v| Value::cell(Value::default_for_suffix(v.name.suffix, defint)))
            .collect();
        self.slot_programs[slot as usize] = Some(LoadedSlot { program, globals });
    }

    /// Whether program SLOT `slot` (0..3) is currently marked executable by `USE` (M6-T6).
    /// Slot 0 (the running program) is always usable; out-of-range slots read `false`.
    pub fn slot_used(&self, slot: u8) -> bool {
        self.slot_used.get(slot as usize).copied().unwrap_or(false)
    }

    /// Replace the file/project store (M6-T2). The native host injects a real-filesystem
    /// [`Storage`] here; the default is an in-memory [`MemStorage`].
    pub fn set_storage(&mut self, storage: Box<dyn Storage>) {
        self.storage = storage;
    }

    /// The current project name (M6-T2).
    pub fn current_project(&self) -> &str {
        &self.current_project
    }

    /// The `ERRNUM` of the last halting error (0 = none) — the DIRECT-mode residue.
    pub fn errnum(&self) -> i32 {
        self.errnum
    }

    /// The `ERRLINE` of the last halting error (the 1-based source line).
    pub fn errline(&self) -> i32 {
        self.errline
    }

    /// The `ERRPRG` of the last halting error (the program SLOT; always 0 in M1).
    pub fn errprg(&self) -> i32 {
        self.errprg
    }

    /// The current `MAINCNT` — the 60 fps frame counter (frames since the clock started).
    pub fn maincnt(&self) -> i32 {
        self.clock.maincnt()
    }

    /// Inject the wall-clock date/time behind `DATE$`/`TIME$` (M6-T3). The native host calls
    /// this with the real RTC; headless it stays at the deterministic epoch. `SYSBEEP`'s flag
    /// (whether the UI beep plays) is read with [`Vm::sysbeep`].
    pub fn set_wall_clock(&mut self, wall_clock: WallClock) {
        self.wall_clock = wall_clock;
    }

    /// The current `SYSBEEP` flag (M6-T3): non-zero = the system UI beep is allowed. The
    /// platform UI layer reads this to decide whether to play the keypress beep.
    pub fn sysbeep(&self) -> i32 {
        self.sysbeep
    }

    /// Read a system variable's live value (M6-T3). Integer sysvars push [`Value::Int`];
    /// `TIME$`/`DATE$` push a [`Value`] String formatted from the injected [`WallClock`].
    fn read_sysvar(&self, sv: Sysvar) -> Value {
        match sv {
            Sysvar::Csrx => Value::Int(self.active_console().cur_x as i32),
            Sysvar::Csry => Value::Int(self.active_console().cur_y as i32),
            // The console is a flat 2-D grid with no per-cursor depth, so CSRZ reads 0.
            Sysvar::Csrz => Value::Int(0),
            Sysvar::Freemem => Value::Int(self.freemem),
            // &H03060000 = 3.6.0 (hw_verified golden, sysvars.yaml).
            Sysvar::Version => Value::Int(0x0306_0000),
            Sysvar::Tabstep => Value::Int(self.tabstep as i32),
            Sysvar::Sysbeep => Value::Int(self.sysbeep),
            Sysvar::Errnum => Value::Int(self.errnum),
            Sysvar::Errline => Value::Int(self.errline),
            Sysvar::Errprg => Value::Int(self.errprg),
            // The PRG* edit target slot defaults to the running slot (refined by PRGEDIT, M6-T4).
            Sysvar::Prgslot => Value::Int(self.current_slot as i32),
            Sysvar::Result => Value::Int(self.result),
            Sysvar::Maincnt => Value::Int(self.clock.maincnt()),
            // Mic/multiplayer are faithful offline stubs (refined in M6-T5). hw_verified offline
            // values (sb-oracle 2026-06-23): mic not recording → MICPOS/MICSIZE = 0; no wireless
            // session → MPCOUNT = 0 but MPHOST/MPLOCAL = -1 (no host / no local user assigned).
            Sysvar::Micpos | Sysvar::Micsize => Value::Int(0),
            Sysvar::Mpcount => Value::Int(0),
            Sysvar::Mphost | Sysvar::Mplocal => Value::Int(-1),
            Sysvar::Time => Value::str_from(&self.wall_clock.time_string()),
            Sysvar::Date => Value::str_from(&self.wall_clock.date_string()),
            Sysvar::Callidx => Value::Int(self.callidx),
        }
    }

    /// Write a *writable* system variable (M6-T3). Only `TABSTEP`/`SYSBEEP` reach here (the
    /// compiler rejects assignment to the read-only ones); the value is coerced to Integer
    /// (a String → Type mismatch, errnum 8). A negative `TABSTEP` clamps to 0 (the tab math is
    /// unsigned); the exact out-of-range behavior is oracle-pending (`bd:sb-interpreter-7td`).
    fn write_sysvar(&mut self, sv: Sysvar, value: Value) -> Result<(), RuntimeError> {
        let n = value.to_int()?;
        match sv {
            Sysvar::Tabstep => self.tabstep = n.max(0) as usize,
            Sysvar::Sysbeep => self.sysbeep = n,
            // Unreachable: the compiler only emits StoreSysvar for the two writable names.
            _ => debug_assert!(false, "StoreSysvar for read-only {}", sv.canonical()),
        }
        Ok(())
    }

    /// The native host's per-frame heartbeat (M4-T3): advance the frame clock one displayed
    /// frame and run the per-frame background machinery (sprite/BG animation step), as the
    /// `swi 0xa` VBlank tick does on hardware. `MAINCNT` advances by one. Called by the
    /// platform's 60 fps loop *after* `run()` returns, so animations set up by the program
    /// keep advancing in the window; it does not touch the VSYNC anchor (only VSYNC/WAIT do).
    pub fn tick_frame(&mut self) {
        self.tick_frames(1);
    }

    /// Advance the frame clock and all frame-driven subsystems by `frames` displayed frames.
    ///
    /// This is the multi-frame form of [`tick_frame`]; interactive hosts that drive the clock
    /// from a real wall-clock 60Hz source use it to catch up after a stall without ticking one
    /// by one. A pending VSYNC/WAIT target is resolved after the jump, and sprite/BG/fader
    /// animation steps run by the full count.
    pub fn tick_frames(&mut self, frames: u64) {
        self.clock.tick(frames);
        // In the interactive model, a pending VSYNC/WAIT target may now be reached.
        self.clock.resolve_wait();
        // Both screens' sprite animations advance every frame (animation runs regardless of
        // which DISPLAY is selected).
        for sp in &mut self.sprites {
            sp.tick(frames);
        }
        // Both screens' BG animations advance every frame, too (same as sprites — #84).
        for bg in &mut self.bg {
            bg.tick(frames);
        }
        // The screen-fader animation advances once per displayed frame.
        self.fader.tick(frames);
    }

    /// Borrow the ACTIVE text console (grid + cursor + colors) for rendering / inspection.
    /// Console commands route here; prefer [`console_for`](Self::console_for) when you need a
    /// specific physical screen.
    pub fn console(&self) -> &Console {
        self.console_for(self.active_screen())
    }

    /// Borrow a specific physical screen's console (0 = Upper, 1 = Touch).
    pub fn console_for(&self, screen_id: usize) -> &Console {
        &self.consoles[screen_id]
    }

    fn active_console(&self) -> &Console {
        self.console_for(self.active_screen())
    }

    fn active_console_mut(&mut self) -> &mut Console {
        let id = self.active_screen();
        &mut self.consoles[id]
    }

    /// Resize the active console to match a new display resolution (e.g. after `XSCREEN`).
    fn resize_active_console(&mut self, width: usize, height: usize) {
        let cols = width / sb_render::console::CELL;
        let rows = height / sb_render::console::CELL;
        self.active_console_mut().resize(cols, rows);
    }

    /// Borrow the GRP graphics state (pages + draw state) for rendering / inspection.
    pub fn grp(&self) -> &GrpState {
        &self.grp
    }

    /// The active DISPLAY screen index (0 = Upper, 1 = Touch) that sprite (and per-screen)
    /// builtins target. Read live from the screen config so it never desyncs from `DISPLAY`.
    fn active_screen(&self) -> usize {
        self.screen.display as usize
    }

    /// Borrow the ACTIVE screen's sprite table (the 512-slot table) for rendering / inspection.
    /// Sprite builtins and SPDEF GET inspect whichever screen `DISPLAY` selects.
    pub fn sprites(&self) -> &SpriteState {
        &self.sprites[self.active_screen()]
    }

    /// Borrow a specific DISPLAY screen's sprite table (0 = Upper, 1 = Touch) — what the
    /// compositor draws for `compose_screen(screen_id, …)`.
    pub fn sprites_for(&self, screen_id: usize) -> &SpriteState {
        &self.sprites[screen_id]
    }

    /// Borrow the shared (global, not per-screen) `SPDEF` definition-template table — for
    /// inspection.
    pub fn spdef(&self) -> &SpdefTable {
        &self.spdef
    }

    /// Borrow the ACTIVE screen's BG system state (the 4-layer tilemap table) for rendering /
    /// inspection. BG builtins inspect whichever screen `DISPLAY` selects.
    pub fn bg(&self) -> &BgState {
        &self.bg[self.active_screen()]
    }

    /// Borrow a specific DISPLAY screen's BG table (0 = Upper, 1 = Touch) — what the compositor
    /// draws for `compose_screen(screen_id, …)`.
    pub fn bg_for(&self, screen_id: usize) -> &BgState {
        &self.bg[screen_id]
    }

    /// Borrow the hardware-input snapshot (button masks + stick axes) — for inspection.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// The Upper-screen layer visibility (`VISIBLE`, M4-T4) for the compositor. The
    /// reimplementation renders only the Upper screen, so this returns screen 0's flags.
    pub fn screen_visibility(&self) -> sb_render::compositor::LayerVisibility {
        self.screen.upper_visibility()
    }

    /// The current fader color as an ARGB code, for `FADE()` / for inspection.
    pub fn fader_color(&self) -> i32 {
        self.fader.current()
    }

    /// The fader color to overlay when rendering, if it is currently enabled (alpha > 0).
    pub fn fader_overlay(&self) -> Option<u32> {
        if self.fader.enabled {
            Some(self.fader.current() as u32)
        } else {
            None
        }
    }

    /// Borrow the VM's screen configuration (`XSCREEN` mode, `DISPLAY` target, `VISIBLE`
    /// flags, `HARDWARE` model). The wasm/native host uses this to pick the right output
    /// dimensions and the active screen's layer visibility.
    pub fn screen_config(&self) -> &ScreenConfig {
        &self.screen
    }

    /// Mutably borrow the hardware-input snapshot so the platform layer (or a scripted-input
    /// test) can advance the frame timeline / fill the button + stick state each frame.
    pub fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    /// True while an interactive `INPUT`/`LINPUT` is waiting for the user to press Enter.
    pub fn awaiting_input(&self) -> bool {
        self.input_wait
    }

    /// The text of the line currently being typed (interactive input only).
    pub fn input_current_line(&self) -> String {
        String::from_utf16_lossy(self.input_buffer.current())
    }

    /// Append one Unicode code point to the interactive input line.
    pub fn input_char(&mut self, ch: u32) {
        self.input_buffer.char(ch);
    }

    /// Append a whole string to the interactive input line (used for IME/paste resync).
    pub fn input_set_current(&mut self, line: &str) {
        self.input_buffer.set_current(line);
    }

    /// Remove the last code point from the interactive input line.
    pub fn input_backspace(&mut self) {
        self.input_buffer.backspace();
    }

    /// Commit the current interactive input line so the next `run_frame` can consume it.
    pub fn input_enter(&mut self) {
        self.input_buffer.enter();
        self.input_wait = false;
    }

    /// The console contents as text: each grid row trimmed of trailing blanks, rows joined
    /// by `\n`, trailing blank rows dropped. This is the deterministic `stdout` of a run
    /// (it mirrors what the oracle scrapes from console memory — e.g. `CLS` empties it).
    pub fn console_text(&self) -> String {
        let c = self.active_console();
        let mut lines: Vec<String> = Vec::with_capacity(c.rows);
        for y in 0..c.rows {
            let mut line = String::new();
            for x in 0..c.cols {
                let ch = c.cell(x, y).ch;
                // An empty cell (never written) reads as a space; trailing ones are
                // trimmed off below.
                line.push(if ch == 0 {
                    ' '
                } else {
                    char::from_u32(ch as u32).unwrap_or('\u{FFFD}')
                });
            }
            lines.push(line.trim_end().to_string());
        }
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Queue one line of input for the next `INPUT`/`LINPUT` (headless input source).
    pub fn push_input(&mut self, line: &str) {
        self.input_buffer.push_line(line);
    }

    /// Run to completion (or error). The operand stack is empty between statements
    /// in well-formed bytecode; a non-empty stack at `End` is tolerated.
    pub fn run(&mut self) -> Result<Halt, VmError> {
        loop {
            if self.pc >= self.program.code.len() {
                // Fell off the end = an implicit `END`; a cross-slot `EXEC` returns here.
                if self.try_exec_return() {
                    continue;
                }
                return Ok(Halt::End);
            }
            let here = self.pc;
            let op = self.program.code[here].clone();
            self.pc += 1;
            match self.step(op) {
                Ok(None) => {}
                Ok(Some(Halt::End)) => {
                    // `END` returns to a cross-slot `EXEC` launcher if one is pending.
                    if self.try_exec_return() {
                        continue;
                    }
                    return Ok(Halt::End);
                }
                Ok(Some(halt)) => return Ok(halt),
                Err(e) => {
                    let e = self.attach_line(e, here);
                    // Capture the error-state residue so ERRNUM/ERRLINE/ERRPRG are
                    // readable after the halt (the DIRECT-mode window, M1-T13). Only a
                    // SmileBASIC runtime error sets it; an `Unsupported` op does not.
                    if let VmError::Sb { errnum, line } = e {
                        self.errnum = errnum as i32;
                        self.errline = line as i32;
                        // ERRPRG = the slot whose code was executing when it halted (M6-T6).
                        // A cross-slot `COMMON DEF` error reports its own slot; the running
                        // program (the common case) is slot 0.
                        self.errprg = self.active_slot as i32;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Run the VM for up to `budget` instructions, yielding when a frame-timing builtin
    /// (`VSYNC`/`WAIT`) asks the host to advance a real frame. Returns `Ok(None)` on yield
    /// (call again next frame), `Ok(Some(Halt::End))` when the program ends, or `Ok(Some(halt))`
    /// on `STOP`/`ASSERT`. This is the execution primitive for interactive hosts that need to
    /// keep the UI responsive while a program polls `BUTTON()` in a `VSYNC` loop.
    pub fn run_frame(&mut self, budget: usize) -> Result<Option<Halt>, VmError> {
        self.interactive = true;
        // While a VSYNC/WAIT target is pending the host drives frames via tick_frame;
        // the program must not resume until the target is reached.
        if self.clock.wait_pending() {
            return Ok(None);
        }
        for _ in 0..budget {
            if self.pc >= self.program.code.len() {
                if self.try_exec_return() {
                    continue;
                }
                return Ok(Some(Halt::End));
            }
            let here = self.pc;
            let op = self.program.code[here].clone();
            self.pc += 1;
            match self.step(op) {
                Ok(None) => {
                    if self.yielded {
                        self.yielded = false;
                        return Ok(None);
                    }
                }
                Ok(Some(Halt::End)) => {
                    if self.try_exec_return() {
                        continue;
                    }
                    return Ok(Some(Halt::End));
                }
                Ok(Some(halt)) => return Ok(Some(halt)),
                Err(e) => {
                    let e = self.attach_line(e, here);
                    if let VmError::Sb { errnum, line } = e {
                        self.errnum = errnum as i32;
                        self.errline = line as i32;
                        self.errprg = self.active_slot as i32;
                    }
                    return Err(e);
                }
            }
        }
        // Budget exhausted without hitting a yield point or halt: return to the host so the
        // platform loop can still paint and gather input.
        Ok(None)
    }

    /// Read a global's current value by name + suffix (for tests / a future REPL).
    pub fn global_value(&self, ident: &str, suffix: Suffix) -> Option<Value> {
        let name = Name::new(ident.to_ascii_uppercase(), suffix);
        let idx = self.program.global_index(&name)? as usize;
        Some(self.globals[idx].borrow().clone())
    }

    // -- dispatch --------------------------------------------------------------

    /// Execute one opcode. `Ok(None)` continues; `Ok(Some(halt))` stops the run.
    fn step(&mut self, op: Op) -> Result<Option<Halt>, VmError> {
        match op {
            Op::Push(c) => self.stack.push(const_to_value(&c)),
            Op::PushVoid => self.stack.push(Value::Void),
            Op::Pop => {
                self.pop()?;
            }

            Op::PushVar(vref) => {
                let v = self.cell(vref)?.borrow().clone();
                self.stack.push(v);
            }
            Op::PushRef(vref) => {
                let cell = self.cell(vref)?.clone();
                self.stack.push(Value::Ref(cell));
            }
            Op::PopVar(vref) => {
                let suffix = self.var_suffix(vref)?;
                let defint = self.program.options.defint;
                let v = self.pop()?.coerce_to_suffix(suffix, defint).map_err(sb)?;
                *self.cell(vref)?.borrow_mut() = v;
            }
            Op::PushSysvar(sv) => {
                let v = self.read_sysvar(sv);
                self.stack.push(v);
            }
            Op::StoreSysvar(sv) => {
                let v = self.pop()?;
                self.write_sysvar(sv, v).map_err(sb)?;
            }

            Op::NewArray { var, ty, dims } => self.new_array(var, ty, dims)?,
            Op::PushArray { var, dims } => self.push_array(var, dims)?,
            Op::PopArray { var, dims } => self.pop_array(var, dims)?,

            Op::Operate(binop) => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                self.stack.push(operate(binop, lhs, rhs).map_err(sb)?);
            }
            Op::OperatePromote(binop) => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                self.stack
                    .push(operate_promote(binop, lhs, rhs).map_err(sb)?);
            }
            Op::Unary(unop) => {
                let v = self.pop()?;
                self.stack.push(unary(unop, v).map_err(sb)?);
            }

            Op::Jump(addr) => self.pc = addr,
            Op::JumpFalse(addr) => {
                if !self.truthy()? {
                    self.pc = addr;
                }
            }
            Op::JumpTrue(addr) => {
                if self.truthy()? {
                    self.pc = addr;
                }
            }
            Op::LogicalAnd(addr) => {
                // Peek: if false, keep it and jump past the rhs; else drop and fall in.
                if !self.peek_truthy()? {
                    self.pc = addr;
                } else {
                    self.pop()?;
                }
            }
            Op::LogicalOr(addr) => {
                if self.peek_truthy()? {
                    self.pc = addr;
                } else {
                    self.pop()?;
                }
            }

            Op::Goto(addr) => self.pc = addr,
            Op::GotoExpr => {
                let addr = self.resolve_code_label()?;
                self.pc = addr;
            }
            Op::Gosub(addr) => {
                self.push_gosub(self.pc)?;
                self.pc = addr;
            }
            Op::GosubExpr => {
                let addr = self.resolve_code_label()?;
                self.push_gosub(self.pc)?;
                self.pc = addr;
            }
            Op::Return => {
                self.pc = self.gosub.pop().ok_or(VmError::Sb {
                    errnum: ERR_RETURN_WITHOUT_GOSUB,
                    line: 0,
                })?;
            }
            Op::OnGoto(targets) => {
                let sel = self.pop()?.to_int().map_err(sb)?;
                if let Some(&addr) = usize::try_from(sel).ok().and_then(|i| targets.get(i)) {
                    self.pc = addr;
                }
            }
            Op::OnGosub(targets) => {
                let sel = self.pop()?.to_int().map_err(sb)?;
                if let Some(&addr) = usize::try_from(sel).ok().and_then(|i| targets.get(i)) {
                    self.push_gosub(self.pc)?;
                    self.pc = addr;
                }
            }

            Op::CallUser {
                name,
                argc,
                out_argc,
                wants_value,
            } => self.call_user(&name, argc, out_argc, wants_value)?,
            Op::ReturnFunc { has_value } => self.return_func(has_value)?,

            Op::CallbackInit(_) => {
                self.callidx = -1;
                self.callback_active = true;
            }
            Op::CallbackNext(kind) => {
                // `self.pc` was advanced past this op by the run loop, so this op's own index
                // is `self.pc - 1`. When a callback is dispatched we rewind there so the sweep
                // resumes (advancing `CALLIDX`) once the callback returns.
                let here = self.pc - 1;
                self.callback_next(kind, here)?;
            }

            Op::ReadValue => {
                let c = self
                    .program
                    .data
                    .get(self.data_cursor)
                    .cloned()
                    .ok_or(VmError::Sb {
                        errnum: ERR_OUT_OF_DATA,
                        line: 0,
                    })?;
                self.data_cursor += 1;
                self.stack.push(const_to_value(&c));
            }
            Op::Restore(idx) => self.data_cursor = idx,
            Op::RestoreExpr => {
                let label = self.pop_label_name()?;
                self.data_cursor = self
                    .program
                    .data_labels
                    .iter()
                    .find(|(n, _)| *n == label)
                    .map(|(_, i)| *i)
                    .ok_or(VmError::Sb {
                        errnum: ERR_UNDEFINED_LABEL,
                        line: 0,
                    })?;
            }

            Op::IncRef => {
                let target = as_ref(self.pop()?)?;
                let delta = self.pop()?;
                let cur = target.deref();
                let new = operate(BinOp::Add, cur, delta).map_err(sb)?;
                target.assign_through(new).map_err(sb)?;
            }
            Op::IncRefPromote => {
                // Real/Number-typed target (or Real delta): overflow promotes Int->Double
                // instead of wrapping, exactly as `A=A+delta` does (hw_verified 2026-06-24).
                let target = as_ref(self.pop()?)?;
                let delta = self.pop()?;
                let cur = target.deref();
                let new = operate_promote(BinOp::Add, cur, delta).map_err(sb)?;
                target.assign_through(new).map_err(sb)?;
            }
            Op::Swap {
                a: sa,
                b: sb_suffix,
            } => {
                // Both operands are references (scalar [`Value::Ref`] cells or array
                // elements [`Value::ElemRef`] — the compiler rejects non-lvalues at
                // parse time). Read BOTH values, then re-coerce each to its target's
                // declared suffix (a typed var truncates/widens like an assignment; an
                // untyped var takes it verbatim) BEFORE writing either — so a
                // Type-mismatch (8) leaves both targets untouched, and an aliased
                // SWAP (same cell / same element) collapses to a no-op.
                let b = as_ref(self.pop()?)?;
                let a = as_ref(self.pop()?)?;
                let va = a.deref();
                let vb = b.deref();
                let defint = self.program.options.defint;
                let into_a = vb.coerce_to_suffix(sa, defint).map_err(sb)?;
                let into_b = va.coerce_to_suffix(sb_suffix, defint).map_err(sb)?;
                a.assign_through(into_a).map_err(sb)?;
                b.assign_through(into_b).map_err(sb)?;
            }

            Op::Raise(errnum) => return Err(sb(RuntimeError::new(errnum))),

            Op::End => return Ok(Some(Halt::End)),
            Op::Stop => return Ok(Some(Halt::Stop)),

            Op::BarewordKeywordErr => {
                // A Class-1 statement keyword used as a sole bareword expression
                // operand (`A=STOP`): real SB reads it as an uninitialized variable
                // name, raising "Uninitialized variable used" (48) at runtime.
                // (STRICT rejects the same form at compile time with 15, so this op
                // only runs in non-STRICT programs.) hw_verified, sb-oracle
                // 2026-06-26, harness/harvest/out/ctm_bareword_kw.tsv.
                return Err(VmError::Sb {
                    errnum: ERR_UNINITIALIZED_VARIABLE,
                    line: 0,
                });
            }

            Op::CallBuiltin {
                name,
                argc,
                out_argc,
                wants_value,
            } => self.call_builtin(&name, argc, out_argc, wants_value)?,

            // -- console output (M1-T8) --------------------------------------------
            Op::PrintItem => {
                let v = self.pop()?.deref();
                let units = crate::builtins::console::format_print_item(&v).map_err(sb)?;
                for u in units {
                    self.active_console_mut().put_char(u);
                }
            }
            Op::PrintTab => {
                let step = self.tabstep;
                self.active_console_mut().tab(step);
            }
            Op::PrintNewline => self.active_console_mut().newline(),
            Op::Input {
                count,
                question,
                has_prompt,
                types,
            } => self.do_input(count, question, has_prompt, &types)?,
            Op::Linput { has_prompt } => self.do_linput(has_prompt)?,

            // `CALL "name"` — dynamic dispatch to a DEF instruction (M6-T6).
            Op::CallDynamic {
                argc,
                out_argc,
                wants_value,
            } => self.call_dynamic(argc, out_argc, wants_value)?,

            // `USE n` / `EXEC target` — program-slot multi-program control (M6-T6).
            Op::Use => {
                let v = self.pop()?;
                self.do_use(v)?;
            }
            Op::Exec => {
                let v = self.pop()?;
                self.do_exec(v)?;
            }

            // -- deferred to later milestones --------------------------------------
            Op::PushRefExpr | Op::PopRef => {
                return Err(VmError::Unsupported("runtime-name reference (VAR())"))
            }
            Op::PushArrayRef { var, dims } => self.push_array_ref(var, dims)?,
        }
        Ok(None)
    }

    // -- arrays ----------------------------------------------------------------

    fn new_array(
        &mut self,
        var: VarRef,
        ty: crate::bytecode::VarType,
        dims: u8,
    ) -> Result<(), VmError> {
        let sizes = self.pop_indices(dims)?;
        let arr = match ty {
            crate::bytecode::VarType::Int => {
                Value::IntArray(SbArray::<i32>::new(&sizes).map_err(sb)?.into_ref())
            }
            crate::bytecode::VarType::Real => {
                Value::RealArray(SbArray::<f64>::new(&sizes).map_err(sb)?.into_ref())
            }
            crate::bytecode::VarType::Str => {
                Value::StrArray(SbArray::<SbStr>::new(&sizes).map_err(sb)?.into_ref())
            }
        };
        *self.cell(var)?.borrow_mut() = arr;
        Ok(())
    }

    fn push_array(&mut self, var: VarRef, dims: u8) -> Result<(), VmError> {
        let idx = self.pop_indices(dims)?;
        let v = match &*self.cell(var)?.borrow() {
            Value::IntArray(a) => Value::Int(a.borrow().get(&idx).map_err(sb)?),
            Value::RealArray(a) => Value::Real(a.borrow().get(&idx).map_err(sb)?),
            Value::StrArray(a) => Value::Str(a.borrow().get(&idx).map_err(sb)?),
            _ => {
                return Err(VmError::Sb {
                    errnum: ERR_TYPE_MISMATCH,
                    line: 0,
                })
            }
        };
        self.stack.push(v);
        Ok(())
    }

    /// Take a reference to array element `var[idx…]` (`SWAP`/`INC`/`DEC`/`OUT`
    /// target). The flat offset is resolved + bounds-checked now (out-of-range →
    /// errnum 31); the resulting [`Value::ElemRef`] shares the array `Rc`, so a
    /// write through it mutates the caller's array.
    fn push_array_ref(&mut self, var: VarRef, dims: u8) -> Result<(), VmError> {
        let idx = self.pop_indices(dims)?;
        let eref = match &*self.cell(var)?.borrow() {
            Value::IntArray(a) => {
                let off = a.borrow().flat_offset(&idx).map_err(sb)?;
                ElemRef::Int(a.clone(), off)
            }
            Value::RealArray(a) => {
                let off = a.borrow().flat_offset(&idx).map_err(sb)?;
                ElemRef::Real(a.clone(), off)
            }
            Value::StrArray(a) => {
                let off = a.borrow().flat_offset(&idx).map_err(sb)?;
                ElemRef::Str(a.clone(), off)
            }
            _ => {
                return Err(VmError::Sb {
                    errnum: ERR_TYPE_MISMATCH,
                    line: 0,
                })
            }
        };
        self.stack.push(Value::ElemRef(eref));
        Ok(())
    }

    fn pop_array(&mut self, var: VarRef, dims: u8) -> Result<(), VmError> {
        let idx = self.pop_indices(dims)?;
        let val = self.pop()?;
        match &*self.cell(var)?.borrow() {
            Value::IntArray(a) => a
                .borrow_mut()
                .set(&idx, val.to_int_store().map_err(sb)?)
                .map_err(sb)?,
            Value::RealArray(a) => a
                .borrow_mut()
                .set(&idx, val.to_real().map_err(sb)?)
                .map_err(sb)?,
            Value::StrArray(a) => a
                .borrow_mut()
                .set(&idx, val.as_str().map_err(sb)?.clone())
                .map_err(sb)?,
            _ => {
                return Err(VmError::Sb {
                    errnum: ERR_TYPE_MISMATCH,
                    line: 0,
                })
            }
        }
        Ok(())
    }

    /// Pop `n` subscripts/sizes, returning them in source order (`[i0, i1, …]`).
    fn pop_indices(&mut self, n: u8) -> Result<Vec<i32>, VmError> {
        let mut out = vec![0i32; n as usize];
        for slot in out.iter_mut().rev() {
            *slot = self.pop()?.to_int().map_err(sb)?;
        }
        Ok(out)
    }

    // -- user functions --------------------------------------------------------

    fn call_user(
        &mut self,
        name: &Name,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        self.invoke_user(name, argc, out_argc, wants_value)
    }

    /// Resolve a user-instruction name to `(target_slot, func_index)` (M6-T6). A function
    /// in the *active* program resolves in-context (`None`); otherwise a `COMMON DEF` in a
    /// `USE`'d, loaded slot resolves cross-slot (`Some(slot)`). A non-`COMMON` `DEF` is
    /// private to its own slot, so it is only found in the active program. `None` → the
    /// name is undefined (errnum 16). Slots are searched in ascending order.
    fn resolve_user_function(&self, name: &Name) -> Option<(Option<u8>, usize)> {
        if let Some(idx) = self.program.function_index(name) {
            return Some((None, idx));
        }
        for slot in 0..self.slot_programs.len() {
            if slot as u8 == self.active_slot || !self.slot_used[slot] {
                continue;
            }
            if let Some(loaded) = &self.slot_programs[slot] {
                if let Some(idx) = loaded.program.function_index(name) {
                    if loaded.program.functions[idx].is_common {
                        return Some((Some(slot as u8), idx));
                    }
                }
            }
        }
        None
    }

    /// Invoke a user instruction by name, switching to its slot's context first when it is
    /// a cross-slot `COMMON DEF` (M6-T6). Shared by the static [`Op::CallUser`] path and
    /// the dynamic [`Op::CallDynamic`] (`CALL "name"`). An unknown name → Undefined
    /// function (16).
    fn invoke_user(
        &mut self,
        name: &Name,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        let (switch, func) = self.resolve_user_function(name).ok_or(VmError::Sb {
            errnum: ERR_UNDEFINED_FUNCTION,
            line: 0,
        })?;
        match switch {
            None => self.invoke_function(func, argc, out_argc, wants_value),
            Some(target) => {
                let caller = self.active_slot;
                self.activate_slot(target);
                self.invoke_function(func, argc, out_argc, wants_value)?;
                // Tag the just-pushed frame so its return swaps the caller's context back.
                if let Some(fr) = self.frames.last_mut() {
                    fr.caller_slot = Some(caller);
                }
                Ok(())
            }
        }
    }

    /// Make program SLOT `target` the active execution context (M6-T6): swap its
    /// program/globals into [`Vm::program`]/[`Vm::globals`] and stash the previously-active
    /// slot's into its slot entry. A no-op when `target` is already active.
    fn activate_slot(&mut self, target: u8) {
        if target == self.active_slot {
            return;
        }
        let prev = self.active_slot as usize;
        let loaded = self.slot_programs[target as usize]
            .take()
            .expect("activate_slot: target slot has no loaded program");
        let prev_program = std::mem::replace(&mut self.program, loaded.program);
        let prev_globals = std::mem::replace(&mut self.globals, loaded.globals);
        self.slot_programs[prev] = Some(LoadedSlot {
            program: prev_program,
            globals: prev_globals,
        });
        self.active_slot = target;
    }

    /// `CALL "name" [,args] [OUT outs]` — dynamic dispatch (M6-T6): resolve a `DEF`
    /// instruction by a **runtime** name string and invoke it exactly like a literal
    /// [`Op::CallUser`]. On entry the operand stack is (bottom→top)
    /// `[name, arg0, …, arg{argc-1}]` — the name string was pushed first (it is the
    /// CALL's first source argument), so it sits *under* the value args. A non-string
    /// name → Type mismatch (8); an unknown instruction → Undefined function (16) — both
    /// hw_verified (`call.yaml`).
    fn call_dynamic(&mut self, argc: u8, out_argc: u8, wants_value: bool) -> Result<(), VmError> {
        // Lift the value args off the top so the name string underneath is reachable.
        let mut args = Vec::with_capacity(argc as usize);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        let name_val = self.pop()?.deref();
        let ident = String::from_utf16_lossy(name_val.as_str().map_err(sb)?).to_ascii_uppercase();
        // A user instruction name carries no type suffix.
        let name = Name::new(ident, Suffix::None);
        // Restore the value args in source order for `invoke_function` to bind.
        for v in args.into_iter().rev() {
            self.stack.push(v);
        }
        // Resolves in the active program or, for a `COMMON DEF`, a `USE`'d slot (M6-T6).
        self.invoke_user(&name, argc, out_argc, wants_value)
    }

    /// Push an activation [`Frame`] for function index `func` and jump to its entry,
    /// binding the `argc` by-value args already on the operand stack. Shared by the
    /// static [`Op::CallUser`] path and the dynamic [`Op::CallDynamic`] (`CALL "name"`).
    fn invoke_function(
        &mut self,
        func: usize,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        if self.frames.len() >= DEF_STACK_LIMIT {
            return Err(VmError::Sb {
                errnum: ERR_STACK_OVERFLOW,
                line: 0,
            });
        }

        let _ = out_argc; // OUT results are produced by ReturnFunc reading out-param locals.
                          // Snapshot the function's local + param types (drops the `program` borrow before
                          // we touch the operand stack below).
        let f = &self.program.functions[func];
        let local_suffixes: Vec<Suffix> = f.locals.iter().map(|v| v.name.suffix).collect();
        let param_suffixes: Vec<Suffix> = f.params.iter().map(|p| p.suffix).collect();

        // Build the frame's locals, each defaulted to its declared type's zero.
        let defint = self.program.options.defint;
        let locals: Vec<Cell> = local_suffixes
            .iter()
            .map(|&s| Value::cell(Value::default_for_suffix(s, defint)))
            .collect();
        // Bind the `argc` by-value args (topmost = last param) into the leading locals,
        // coercing each to its parameter's static type. Surplus args are dropped.
        let defint = self.program.options.defint;
        for i in (0..argc as usize).rev() {
            let v = self.pop()?;
            if let Some(&suffix) = param_suffixes.get(i) {
                *locals[i].borrow_mut() = v.coerce_to_suffix(suffix, defint).map_err(sb)?;
            }
        }

        self.frames.push(Frame {
            locals,
            return_pc: self.pc,
            func,
            wants_value,
            // A cross-slot call retags this after the push (see `invoke_user`).
            caller_slot: None,
        });
        self.pc = self.program.functions[func].address;
        Ok(())
    }

    /// `USE` — mark a program SLOT executable (M6-T6). Two operand forms, both `hw_verified`
    /// (sb-oracle 2026-06-23):
    /// * **numeric slot** `USE n`: `n` outside 0..3 → Out of range (10); `n` == the *running*
    ///   slot (you cannot `USE` the slot you are executing) → Illegal function call (4); a
    ///   valid non-running slot is marked executable.
    /// * **resource string** `USE "PRGn:file"`: an unknown resource type / a PRG index past the
    ///   family (`PRG4`/`PRG5`) / an empty name → 4 (note: NOT 10 like the `SAVE` resolver — the
    ///   slot machinery rejects an out-of-family PRG index as an unknown resource); the running
    ///   slot (`PRG0`) → 4; a missing file → Load failed (46); an existing file marks the slot.
    ///
    /// Loading the compiled program into the slot — so its DEFs/labels resolve from a
    /// cross-slot `CALL`/`GOSUB` — is the remaining multi-program model (queued, `bd:sb-interpreter-air`).
    /// Compile a stored program file into a loadable [`Program`] for a slot (M6-T6
    /// string-form `EXEC`/`USE`). The TXT body is read from the current project's storage
    /// (UTF-8, like `LOAD "TXT:"`), parsed, and lowered with the standard builtin set — the
    /// in-VM `parse`→`compile_with` pipeline, so no external compile hook is needed. The
    /// caller has already confirmed the file exists. A program that fails to parse/compile
    /// maps to Syntax error (3) — the exact errnum for a malformed stored program is
    /// oracle-pending (queued, `bd:sb-interpreter-c9d`).
    fn compile_slot_file(&self, name: &str) -> Result<Program, VmError> {
        let body = self
            .storage
            .read(&self.current_project, Folder::Txt, name)
            .map_err(|_| sb(RuntimeError::new(ERR_LOAD_FAILED)))?;
        let src = String::from_utf8_lossy(&body);
        let ast = crate::parser::parse(&src)
            .map_err(|_| sb(RuntimeError::new(crate::builtins::ERR_SYNTAX)))?;
        crate::compiler::compile_with(&ast, &crate::builtins::StdBuiltins)
            .map_err(|_| sb(RuntimeError::new(crate::builtins::ERR_SYNTAX)))
    }

    fn do_use(&mut self, v: Value) -> Result<(), VmError> {
        if let Value::Str(s) = &v {
            let s = String::from_utf16_lossy(s);
            let (slot_opt, name) =
                parse_prg_operand(&s).map_err(|errnum| sb(RuntimeError::new(errnum)))?;
            // A bare filename (no `PRGn:` resource number) defaults to the RUNNING slot — the
            // same rule EXEC uses (osb `Exec.execute`: `if (!file.hasResourceNumber)
            // file.resourceNumber = currentSlotNumber`). Since you cannot USE the slot you are
            // executing, a bare name therefore always hits the running-slot guard → errnum 4,
            // and that guard precedes the file-existence check (hw_verified, sb-oracle
            // 2026-06-23: `USE "NOPE"` / `USE "Q"` / `USE "abc"` — bare, missing file — all
            // raise 4, NOT the 46 a missing `PRGn:` file gives).
            let slot = slot_opt.unwrap_or(self.current_slot);
            if slot == self.current_slot {
                return Err(sb(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL)));
            }
            if !self
                .storage
                .exists(&self.current_project, Folder::Txt, name)
            {
                return Err(sb(RuntimeError::new(ERR_LOAD_FAILED)));
            }
            // Load the slot's program from storage so its `COMMON DEF`s become resolvable
            // cross-slot (the documented effect of marking a slot executable). Uniform across
            // slots 0..3: the running-slot guard above (`slot == current_slot` → errnum 4) means
            // slot 0 is reachable here only when it is *non-running* (parked in
            // `slot_programs[0]`), so it loads the same way as any other slot (osb keeps all
            // SLOTs in one `slots[]` array).
            let prog = self.compile_slot_file(name)?;
            self.stash_slot_program(slot, prog);
            self.slot_used[slot as usize] = true;
            Ok(())
        } else {
            let n = v.to_int().map_err(sb)?;
            if !(0..=3).contains(&n) {
                return Err(sb(RuntimeError::new(ERR_OUT_OF_RANGE)));
            }
            let slot = n as u8;
            if slot == self.current_slot {
                return Err(sb(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL)));
            }
            self.slot_used[slot as usize] = true;
            Ok(())
        }
    }

    /// `EXEC` — load and/or execute another program SLOT (M6-T6). The validation is
    /// `hw_verified` (sb-oracle 2026-06-23); the actual control transfer is the deferred
    /// multi-program model:
    /// * **numeric slot** `EXEC n`: `n` outside 0..3 → Out of range (10); a valid *non-running*
    ///   slot with a loaded program transfers control to it ([`Vm::exec_transfer`], documented);
    ///   an *empty* non-running slot → Syntax error (3) (`EXEC 1` on a fresh, empty slot); the
    ///   *running* slot restarts the current program ([`Vm::restart_active_slot`]).
    /// * **resource string** `EXEC "PRGn:file"`: an unknown resource type / bad PRG index /
    ///   empty name → 4; a missing file → Load failed (46); an existing file in a `PRGn:` slot
    ///   *distinct* from the running one is read from storage, compiled in-VM, loaded into that
    ///   slot, and run ([`Vm::compile_slot_file`] + [`Vm::exec_transfer`], documented form 1);
    ///   a `PRGn:` slot naming the *running* slot loads the file into the running slot and runs
    ///   it from the top ([`Vm::load_into_running_slot`], the corpus loader idiom); a *bare name*
    ///   (no `PRGn:` resource number) defaults to the running slot (osb `Exec.execute`) and is
    ///   loaded + run the same way.
    ///
    /// The numeric loaded-slot transfer, the numeric running-slot restart, the string-form
    /// `PRGn:` file LOAD (into a non-running OR the running slot — including a non-running
    /// slot 0, uniform with the others), and the bare-name default-slot LOAD are the
    /// documented single-level model. Per-slot vs shared variable scoping, and the exact
    /// resume-state granularity preserved when a loaded slot's `END` returns to a launcher
    /// whose own slot was overwritten by the load (the slot-0 clobber edge), stay
    /// oracle-pending (queued, `bd:sb-interpreter-air`).
    fn do_exec(&mut self, v: Value) -> Result<(), VmError> {
        if let Value::Str(s) = &v {
            let s = String::from_utf16_lossy(s);
            let (slot_opt, name) =
                parse_prg_operand(&s).map_err(|errnum| sb(RuntimeError::new(errnum)))?;
            if !self
                .storage
                .exists(&self.current_project, Folder::Txt, name)
            {
                return Err(sb(RuntimeError::new(ERR_LOAD_FAILED)));
            }
            match slot_opt {
                // A `PRGn:` slot distinct from the running one: load the file's program into
                // that slot and transfer control to it (documented form 1, "Loads and
                // executes a program"). Uniform across slots 0..3 — a `PRG0:` resource naming
                // a *non-running* slot 0 loads into slot 0's parked registry entry the same
                // way (osb keeps all SLOTs in one `slots[]` array — slot 0 is not special).
                Some(slot) if slot != self.current_slot => {
                    let prog = self.compile_slot_file(name)?;
                    self.stash_slot_program(slot, prog);
                    self.exec_transfer(slot);
                    Ok(())
                }
                // A `PRGn:` slot naming the RUNNING slot (`EXEC "PRG0:file"` while slot 0 runs)
                // OR a bare filename with no `PRGn:` resource number (which defaults to the
                // running slot — osb `Exec.execute`: `if (!file.hasResourceNumber)
                // file.resourceNumber = currentSlotNumber`; the corpus loader idiom
                // `EXEC EXE$` / `EXEC FAV$[...]`): load the file's program INTO the running
                // slot and execute it from the top — documented form 1 applied to the running
                // slot. Like a restart, the previous program is abandoned (impossible to
                // return to it).
                Some(_) | None => {
                    let prog = self.compile_slot_file(name)?;
                    self.load_into_running_slot(prog);
                    Ok(())
                }
            }
        } else {
            let n = v.to_int().map_err(sb)?;
            if !(0..=3).contains(&n) {
                return Err(sb(RuntimeError::new(ERR_OUT_OF_RANGE)));
            }
            if n as u8 == self.current_slot {
                // `EXEC <running slot>` (e.g. `EXEC 0`) restarts the running program from the
                // top — the corpus "restart the game" idiom (`EXEC 0` in 1DVK34J/TXT/HNZBUS,
                // `IF DIALOG(...)<0 THEN END ELSE EXEC 0`). Documented "Loads and executes a
                // program"; since you cannot return to the previous program, this is a fresh
                // re-run of the same slot.
                self.restart_active_slot();
                return Ok(());
            }
            // A valid non-running slot. If a program is loaded there (host/test seeded it via
            // `Vm::load_slot_program`, or a future `LOAD "PRGn:…"`), EXEC transfers control to
            // it — documented "Executes a program in a different SLOT". An *empty* slot raises
            // Syntax error (3) (hw_verified: `EXEC 1` on a fresh, empty slot).
            if self.slot_programs[n as usize].is_some() {
                self.exec_transfer(n as u8);
                Ok(())
            } else {
                Err(sb(RuntimeError::new(crate::builtins::ERR_SYNTAX)))
            }
        }
    }

    /// `EXEC` control transfer (M6-T6) — switch the running program to a loaded, non-running
    /// slot and begin executing it from the top. This is the *documented* model (exec.yaml):
    /// "Executes a program in a different SLOT". Because it is impossible to return to the
    /// previous program, the caller's whole execution state — `DEF` frames, the `GOSUB`
    /// stack, the operand stack, and the `DATA` read cursor — is discarded; the target slot
    /// runs against its own globals (swapped in by [`Vm::activate_slot`]). When the EXEC'd
    /// program ends, the run ends.
    ///
    /// Per the documented cross-slot return rule, the launcher's full execution state is
    /// captured into [`Vm::exec_returns`] before the swap, so that an `END` in the EXEC'd
    /// program (handled by [`Vm::try_exec_return`]) hands control back to the launcher. The
    /// running-slot *restart* is [`Vm::restart_active_slot`]. DIRECT-mode gating remains
    /// oracle-pending (queued, `bd:sb-interpreter-7td`).
    fn exec_transfer(&mut self, target: u8) {
        // Capture the launcher's resume state for the documented END-returns rule (the `pc`
        // already points past the EXEC statement). `target != active_slot` always holds here.
        self.exec_returns.push(ExecReturn {
            slot: self.active_slot,
            pc: self.pc,
            stack: std::mem::take(&mut self.stack),
            frames: std::mem::take(&mut self.frames),
            gosub: std::mem::take(&mut self.gosub),
            data_cursor: self.data_cursor,
        });
        self.activate_slot(target);
        self.current_slot = target;
        self.pc = 0;
        self.data_cursor = 0;
    }

    /// The documented cross-slot `EXEC` return (M6-T6): when the active program reaches `END`
    /// (or falls off the end of its code), if a cross-slot `EXEC` is pending its launcher is
    /// restored and `true` is returned so the run continues in the launcher; otherwise `false`
    /// (a genuine halt). This realises "It is possible to return by using END in a program
    /// started with EXEC in another SLOT" (exec.md). The launcher's program/globals are swapped
    /// back via [`Vm::activate_slot`] and its pc/stack/frames/gosub/data-cursor restored.
    /// `STOP` does NOT trigger a return (only `END` / end-of-code does), matching the docs.
    fn try_exec_return(&mut self) -> bool {
        let Some(ret) = self.exec_returns.pop() else {
            return false;
        };
        self.activate_slot(ret.slot);
        self.current_slot = ret.slot;
        self.pc = ret.pc;
        self.stack = ret.stack;
        self.frames = ret.frames;
        self.gosub = ret.gosub;
        self.data_cursor = ret.data_cursor;
        true
    }

    /// `EXEC <running slot>` restart (M6-T6) — re-run the active program from the top. Unlike
    /// [`Vm::exec_transfer`] there is no slot swap (the target *is* the running program); the
    /// program's globals are re-initialised to their declared-type zeros — a fresh run, the
    /// documented "Loads and executes a program" — and all execution state (the `DEF` frame
    /// stack, `GOSUB` stack, operand stack, and `DATA` read cursor) is discarded before the PC
    /// jumps to the top. This mirrors what re-running re-executes (`DIM`/variable init), the
    /// only coherent meaning of a restart: a restart that kept old variable values would just
    /// be a `GOTO @TOP`. Per-slot I/O state (graphics/sprites/console/system variables) is not
    /// touched, matching `exec_transfer`.
    ///
    /// Grounded on docs + the corpus restart idiom (no body-readable handler, single-`P` oracle
    /// can't drive a self-restart without a hang); whether real SB clears variables on `EXEC 0`
    /// vs preserves them is queued for a multi-slot oracle confirm (`bd:sb-interpreter-air`).
    fn restart_active_slot(&mut self) {
        let defint = self.program.options.defint;
        self.globals = self
            .program
            .globals
            .iter()
            .map(|v| Value::cell(Value::default_for_suffix(v.name.suffix, defint)))
            .collect();
        self.pc = 0;
        self.frames.clear();
        self.gosub.clear();
        self.stack.clear();
        self.data_cursor = 0;
    }

    /// `EXEC "PRGn:file"` into the RUNNING slot (M6-T6) — replace the currently-executing
    /// program with one freshly loaded from storage and run it from the top. This is the
    /// documented form 1 ("Loads and executes a program", exec.yaml) applied to the running
    /// slot: `EXEC "PRG0:file"` while slot 0 runs — the corpus loader idiom where a small
    /// slot-0 loader `LOAD`s/`EXEC`s the real program. The loaded program *replaces* the
    /// running one ([`Vm::program`]/[`Vm::globals`]); because it is impossible to return to
    /// the previous program the whole execution state (`DEF` frames, the `GOSUB` stack, the
    /// operand stack, and the `DATA` read cursor) is discarded and the new program's globals
    /// start at their declared-type zeros. Per-slot I/O state (graphics/sprites/console/
    /// system variables) is untouched, matching [`Vm::exec_transfer`]/[`Vm::restart_active_slot`].
    /// The slot number is unchanged (the file loads into the slot already running).
    fn load_into_running_slot(&mut self, program: Program) {
        let defint = program.options.defint;
        self.globals = program
            .globals
            .iter()
            .map(|v| Value::cell(Value::default_for_suffix(v.name.suffix, defint)))
            .collect();
        self.program = program;
        self.pc = 0;
        self.frames.clear();
        self.gosub.clear();
        self.stack.clear();
        self.data_cursor = 0;
    }

    /// Call a registered builtin (M1-T7 math/string set). Pops `argc` value args
    /// (topmost = last arg), dispatches by canonical name, and pushes the single return
    /// value when the caller wants it. These builtins take no `OUT` params, so
    /// `out_argc` is expected to be 0. An unknown name → Undefined function (errnum 16).
    fn call_builtin(
        &mut self,
        name: &str,
        argc: u8,
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), VmError> {
        let mut args = Vec::with_capacity(argc as usize);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();
        // RNG builtins (RND/RNDF/RANDOMIZE, M1-T9) read/mutate the VM's TinyMT series, so
        // they can't go through the stateless `builtins::dispatch`.
        if let Some(ret) = self.call_rng(name, &args).map_err(sb)? {
            if wants_value {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // `ASSERT__` (M1-T14) is the test-mode builtin: a false condition halts the run
        // with [`VmError::Assert`] (not a SmileBASIC error). It is a statement command, so
        // it produces no value.
        if name == "ASSERT__" {
            self.call_assert(&args)?;
            return Ok(());
        }
        // Frame-timing builtins (WAIT/VSYNC, M4-T3): advance the frame clock (instantly in
        // the headless model) after validating the documented call shape.
        if let Some(ret) = self.call_timing(name, &args, wants_value).map_err(sb)? {
            if wants_value && !matches!(ret, Value::Void) {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // Array data-ops (SORT/RSORT, M1-T14) reorder their array arguments in place —
        // each arrives as a shared `ArrayRef`, so they mutate the caller's variables and
        // produce no value.
        if matches!(name, "SORT" | "RSORT") {
            crate::builtins::data::sort(&args, name == "RSORT").map_err(sb)?;
            return Ok(());
        }
        // Block ops (COPY/FILL, M1-T14). COPY reads from a source array or — in form 2,
        // `COPY dest,"@Label"` — from the program's DATA pool, so it lives in the VM
        // (`call_copy`) where the DATA pool is reachable; FILL is a pure array write.
        if name == "COPY" {
            self.call_copy(&args).map_err(sb)?;
            return Ok(());
        }
        if name == "FILL" {
            crate::builtins::data::fill(&args).map_err(sb)?;
            return Ok(());
        }
        // Stack/queue ops (PUSH/UNSHIFT grow, POP/SHIFT shrink, M1-T14). The operand is a
        // shared `ArrayRef` (array form) or a `Value::Ref` to a string scalar, so they
        // mutate the caller's variable; POP/SHIFT also yield the removed element.
        if matches!(name, "PUSH" | "UNSHIFT") {
            crate::builtins::data::push(&args, name == "UNSHIFT").map_err(sb)?;
            return Ok(());
        }
        if matches!(name, "POP" | "SHIFT") {
            let ret = crate::builtins::data::pop(&args, name == "SHIFT").map_err(sb)?;
            if wants_value {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // Console builtins (LOCATE/COLOR/CLS/ACLS/BACKCOLOR/INKEY$, M1-T8) mutate the
        // VM-owned console / screen state, so they too sidestep the stateless dispatch.
        if let Some(ret) = self.call_console(name, &args, wants_value).map_err(sb)? {
            if wants_value && !matches!(ret, Value::Void) {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // Graphics builtins (GPAGE/GCLS/GCOLOR/GPRIO/GCLIP/RGB/RGBREAD/GSPOIT, M2-T1) mutate
        // or read the VM-owned `GrpState` and can leave OUT results, so they push their own
        // results and bypass the stateless dispatch.
        if self.call_graphics(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        // Sprite lifecycle builtins (SPSET/SPCLR/SPSHOW/SPHIDE/SPUSED, M3-T1) mutate or read
        // the VM-owned `SpriteState` and can leave OUT results, so they push their own
        // results and bypass the stateless dispatch.
        if self.call_sprite(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        // BG core builtins (BGSCREEN/BGPUT/BGGET/BGFILL/BGCLR + the per-layer transforms,
        // M3-T4) mutate or read the VM-owned `BgState` and can leave OUT results, so they
        // push their own results and bypass the stateless dispatch.
        if self.call_bg(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        // Hardware-input builtins (BUTTON/STICK/STICKEX/BREPEAT, M4-T1) read/mutate the
        // VM-owned `InputState` and can leave OUT results, so they push their own results
        // and bypass the stateless dispatch.
        if self.call_input(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        // Screen configuration (XSCREEN/DISPLAY/VISIBLE/HARDWARE, M4-T4) mutates/reads the
        // VM-owned `ScreenConfig`, so it bypasses the stateless dispatch.
        if let Some(ret) = self.call_screen(name, &args, wants_value).map_err(sb)? {
            if wants_value && !matches!(ret, Value::Void) {
                self.stack.push(ret);
            }
            return Ok(());
        }
        // BGM commands (BGMPLAY/…/BGMCLEAR, M5-T3) manage the VM-owned `AudioState` (and, for
        // BGMSETD, read MML from the program's DATA pool), so they bypass the stateless
        // dispatch. They push their own result (BGMCHK / the BGMVAR read form).
        if self.call_sound(name, &args, wants_value)? {
            return Ok(());
        }
        // File commands (SAVE/LOAD/FILES/DELETE/RENAME/CHKFILE/PROJECT, M6-T2) operate on the
        // VM-owned `Storage` + current project, and can read/write array/OUT operands or push a
        // value (CHKFILE / LOAD function form), so they bypass the stateless dispatch.
        if self.call_files(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        // Source-edit family (PRGEDIT/PRGGET$/PRGSET/PRGINS/PRGDEL/PRGNAME$/PRGSIZE, M6-T4)
        // read/mutate the VM-owned program-slot source + edit-target state, and the function
        // forms push their own value, so they bypass the stateless dispatch.
        if self.call_prg(name, &args, wants_value)? {
            return Ok(());
        }
        // Faithful limitation stubs (M6-T5): XON/XOFF feature gate, the microphone (MIC*),
        // motion sensors (GYRO*/ACCEL), wireless multiplayer (MP*) and DIALOG. They read/mutate
        // the VM-owned `DeviceState` + `RESULT`, push their own value/OUT results, and reproduce
        // the disassembled arg-shape / range / availability (36/37) guards.
        if self.call_device(name, &args, out_argc, wants_value)? {
            return Ok(());
        }
        let ret = crate::builtins::dispatch(name, args).map_err(sb)?;
        if wants_value {
            self.stack.push(ret);
        }
        Ok(())
    }

    /// `COPY dest [,dest_offset], src [[,src_offset], count]` (form 1, array→array) or
    /// `COPY dest [,dest_offset], "@Label" [,count]` (form 2, DATA→array). The form and
    /// the optional offsets are disambiguated by argument **type**: a numeric in the
    /// second slot is `dest_offset`; the source operand is then an array (form 1) or a
    /// string `"@Label"` (form 2). For 1D destinations the array auto-extends if too
    /// small. Errors (hw_verified sb-oracle 2026-06-22, s_t4c): a non-array source/dest
    /// or a numeric↔string element mismatch → Type mismatch (8); too few/many arguments
    /// → Illegal function call (4); an out-of-range offset/count → Out of range (10);
    /// form 2 with an undefined label → Undefined label (14); form 2 with fewer DATA
    /// items than required → Out of DATA (13).
    fn call_copy(&mut self, args: &[Value]) -> Result<(), RuntimeError> {
        use crate::builtins::data::{elem_count, is_numeric, nonneg, read_values, write_values};
        use crate::builtins::illegal;
        let dest = args.first().ok_or_else(illegal)?;
        let mut i = 1;
        // An optional `dest_offset` only when a numeric is followed by the source operand.
        let dest_offset = if i + 1 < args.len() && is_numeric(&args[i]) {
            let off = nonneg(&args[i])?;
            i += 1;
            off
        } else {
            0
        };
        let src = args.get(i).ok_or_else(illegal)?;
        let trailing = &args[i + 1..];

        if let Value::Str(label) = src {
            // Form 2 — read a DATA sequence named by "@Label" into the destination.
            let count = match trailing {
                [] => elem_count(dest)?,
                [c] => nonneg(c)?,
                _ => return Err(illegal()),
            };
            let name = String::from_utf16_lossy(label)
                .trim_start_matches('@')
                .to_ascii_uppercase();
            let idx = self
                .program
                .data_labels
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, i)| *i)
                .ok_or_else(|| RuntimeError::new(ERR_UNDEFINED_LABEL))?;
            let mut vals = Vec::with_capacity(count);
            for k in 0..count {
                let c = self
                    .program
                    .data
                    .get(idx + k)
                    .ok_or_else(|| RuntimeError::new(ERR_OUT_OF_DATA))?;
                vals.push(const_to_value(c));
            }
            write_values(dest, dest_offset, &vals, true)
        } else {
            // Form 1 — copy elements from a source array.
            let (src_offset, count) = match trailing {
                [] => (0, None),
                [c] => (0, Some(nonneg(c)?)),
                [so, c] => (nonneg(so)?, Some(nonneg(c)?)),
                _ => return Err(illegal()),
            };
            let src_len = elem_count(src)?;
            let count = count.unwrap_or_else(|| src_len.saturating_sub(src_offset));
            let vals = read_values(src, src_offset, count)?;
            write_values(dest, dest_offset, &vals, true)
        }
    }

    /// File commands (M6-T2) over the VM-owned [`Storage`](crate::storage::Storage) + current
    /// project. Returns `Ok(true)` when `name` is a file command (handled — pushing any value
    /// itself), `Ok(false)` otherwise (the caller falls through to the stateless dispatch).
    /// Argument-shape / type / load-failure errnums follow the
    /// `spec/instructions/{save,load,files,delete,rename,chkfile,project}.yaml` contracts.
    fn call_files(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "SAVE" => self.file_save(&args, wants_value).map_err(sb)?,
            "LOAD" => self.file_load(&args, out_argc, wants_value).map_err(sb)?,
            "FILES" => self.file_files(&args, wants_value).map_err(sb)?,
            "DELETE" => self.file_delete(&args, wants_value).map_err(sb)?,
            "RENAME" => self.file_rename(&args, wants_value).map_err(sb)?,
            "CHKFILE" => self.file_chkfile(&args, wants_value).map_err(sb)?,
            "PROJECT" => self
                .file_project(&args, out_argc, wants_value)
                .map_err(sb)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `SAVE "[Resource:]Name"[, data]` — write a resource. Statement-only and ≥1 arg (else
    /// Syntax error 3); the first operand must be a string (else Type mismatch 8). `TXT:` takes
    /// a string data operand (UTF-8 body), `DAT:` a numeric-array operand. Program-slot /
    /// graphic / font (form 1) record an empty body for now (payload plumbing queued, O-T3).
    fn file_save(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        use crate::builtins::files::{encode_dat, encode_txt, resolve_kind, storage_errnum};
        use crate::builtins::ERR_SYNTAX;
        if wants_value || args.is_empty() {
            return Err(RuntimeError::new(ERR_SYNTAX));
        }
        let name = String::from_utf16_lossy(args[0].as_str()?);
        let (spec, fname) =
            parse_resource(&name).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let kind = resolve_kind(spec, self.current_slot);
        let body = match kind {
            ResourceKind::Data => {
                let arr = args.get(1).ok_or_else(|| RuntimeError::new(ERR_SYNTAX))?;
                encode_dat(arr)?
            }
            ResourceKind::Text => match args.get(1) {
                Some(v) => encode_txt(v.as_str()?),
                None => Vec::new(),
            },
            _ => Vec::new(),
        };
        self.storage
            .write(&self.current_project, kind.folder(), fname, &body)
            .map_err(|e| storage_errnum(&e))
    }

    /// `LOAD "[Resource:]Name"[, …]` — read a resource. ≥1 arg (else Illegal function call 4);
    /// the first operand must be a string (else Type mismatch 8). `TXT:` returns the text as a
    /// string (function form, or the `OUT` target); `DAT:` reads into the numeric-array
    /// operand; program/graphic/font (form 1) confirm existence (Load failed 46 if missing).
    fn file_load(
        &mut self,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), RuntimeError> {
        use crate::builtins::files::{decode_dat_into, decode_txt, resolve_kind, storage_errnum};
        use crate::builtins::ERR_ILLEGAL_FUNCTION_CALL;
        if args.is_empty() {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let name = String::from_utf16_lossy(args[0].as_str()?);
        let (spec, fname) =
            parse_resource(&name).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let kind = resolve_kind(spec, self.current_slot);
        match kind {
            ResourceKind::Text => {
                let body = self
                    .storage
                    .read(&self.current_project, Folder::Txt, fname)
                    .map_err(|e| storage_errnum(&e))?;
                if wants_value || out_argc == 1 {
                    self.stack.push(Value::Str(decode_txt(&body)));
                }
            }
            ResourceKind::Data => {
                let dest = args
                    .get(1)
                    .ok_or_else(|| RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL))?;
                let body = self
                    .storage
                    .read(&self.current_project, Folder::Dat, fname)
                    .map_err(|e| storage_errnum(&e))?;
                decode_dat_into(dest, &body)?;
            }
            _ => {
                // Program slot / graphic page / font page (form 1): confirm existence
                // (Load failed 46 if missing); restoring into the slot/page is queued (O-T3).
                self.storage
                    .read(&self.current_project, kind.folder(), fname)
                    .map_err(|e| storage_errnum(&e))?;
            }
        }
        Ok(())
    }

    /// `FILES ["filter"][, strArray$]` — list files. Statement-only. One operand is either a
    /// filter string or an output string array; two operands are filter + output array; a
    /// wrong operand type is Type mismatch (8), more than two operands Syntax error (3). With
    /// an output array the names fill it (1-D auto-extends); otherwise they list to the console.
    fn file_files(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        use crate::builtins::files::storage_errnum;
        use crate::builtins::ERR_SYNTAX;
        if wants_value {
            return Err(RuntimeError::new(ERR_SYNTAX));
        }
        let (filter, out_array): (FilesFilter, Option<&Value>) = match args {
            [] => (FilesFilter::All, None),
            [a] => match a {
                Value::Str(s) => (parse_files_filter(&String::from_utf16_lossy(s)), None),
                Value::StrArray(_) => (FilesFilter::All, Some(a)),
                _ => return Err(RuntimeError::new(crate::builtins::ERR_TYPE_MISMATCH)),
            },
            [f, arr] => {
                let s = f.as_str()?;
                if !matches!(arr, Value::StrArray(_)) {
                    return Err(RuntimeError::new(crate::builtins::ERR_TYPE_MISMATCH));
                }
                (parse_files_filter(&String::from_utf16_lossy(s)), Some(arr))
            }
            _ => return Err(RuntimeError::new(ERR_SYNTAX)),
        };
        let names = self.files_list(&filter).map_err(|e| storage_errnum(&e))?;
        if let Some(Value::StrArray(a)) = out_array {
            let mut a = a.borrow_mut();
            let _ = a.resize(names.len());
            let n = a.len().min(names.len());
            let slice = a.as_mut_slice();
            for (i, nm) in names.iter().take(n).enumerate() {
                slice[i] = nm.encode_utf16().collect();
            }
        } else {
            for nm in &names {
                for u in nm.encode_utf16() {
                    self.active_console_mut().put_char(u);
                }
                self.active_console_mut().newline();
            }
        }
        Ok(())
    }

    /// Resolve a [`FilesFilter`] to the sorted names it lists in the current project (or, for
    /// the project-list / named-project filters, across projects).
    fn files_list(
        &self,
        filter: &FilesFilter,
    ) -> Result<Vec<String>, crate::storage::StorageError> {
        let both = |proj: &str| -> Result<Vec<String>, crate::storage::StorageError> {
            let mut names = self.storage.list(proj, Folder::Txt)?;
            names.extend(self.storage.list(proj, Folder::Dat)?);
            names.sort();
            names.dedup();
            Ok(names)
        };
        match filter {
            FilesFilter::All => both(&self.current_project),
            FilesFilter::Txt => self.storage.list(&self.current_project, Folder::Txt),
            FilesFilter::Dat => self.storage.list(&self.current_project, Folder::Dat),
            FilesFilter::Projects => self.storage.projects(),
            FilesFilter::Project(p) => both(p),
        }
    }

    /// `DELETE "[Filetype:]Name"` — delete a file. Statement-only and exactly 1 arg (else
    /// Syntax error 3); the operand must be a string (else Type mismatch 8). Deleting a
    /// missing file is a no-op (no error).
    fn file_delete(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        use crate::builtins::files::{resolve_kind, storage_errnum};
        use crate::builtins::ERR_SYNTAX;
        if wants_value || args.len() != 1 {
            return Err(RuntimeError::new(ERR_SYNTAX));
        }
        let name = String::from_utf16_lossy(args[0].as_str()?);
        let (spec, fname) =
            parse_resource(&name).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let kind = resolve_kind(spec, self.current_slot);
        self.storage
            .delete(&self.current_project, kind.folder(), fname)
            .map_err(|e| storage_errnum(&e))?;
        Ok(())
    }

    /// `RENAME "[Resource:]Name","[Resource:]New"` — rename a file. Statement-only and exactly
    /// 2 args (else Syntax error 3); both operands must be strings (the first non-string → Type
    /// mismatch 8). The rename stays within the source resource's folder (cross-resource
    /// retype is corpus-only / queued).
    fn file_rename(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        use crate::builtins::files::{resolve_kind, storage_errnum};
        use crate::builtins::ERR_SYNTAX;
        if wants_value || args.len() != 2 {
            return Err(RuntimeError::new(ERR_SYNTAX));
        }
        let from = String::from_utf16_lossy(args[0].as_str()?);
        let to = String::from_utf16_lossy(args[1].as_str()?);
        let (fspec, fname) =
            parse_resource(&from).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let (_tspec, tname) =
            parse_resource(&to).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let folder = resolve_kind(fspec, self.current_slot).folder();
        self.storage
            .rename(&self.current_project, folder, fname, tname)
            .map_err(|e| storage_errnum(&e))?;
        Ok(())
    }

    /// `CHKFILE("[Resource:]Name")` → `TRUE`/`FALSE` for existence. Function-only (read for a
    /// value) and exactly 1 arg (else Illegal function call 4); the operand must be a string
    /// (else Type mismatch 8). Pushes the boolean result.
    fn file_chkfile(&mut self, args: &[Value], wants_value: bool) -> Result<(), RuntimeError> {
        use crate::builtins::files::resolve_kind;
        use crate::builtins::ERR_ILLEGAL_FUNCTION_CALL;
        if !wants_value || args.len() != 1 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let name = String::from_utf16_lossy(args[0].as_str()?);
        let (spec, fname) =
            parse_resource(&name).map_err(|e| RuntimeError::new(e.errnum() as u32))?;
        let kind = resolve_kind(spec, self.current_slot);
        let exists = self
            .storage
            .exists(&self.current_project, kind.folder(), fname);
        self.stack.push(Value::Int(i32::from(exists)));
        Ok(())
    }

    /// `PROJECT "name"` (set) / `PROJECT OUT name$` (read). The **set** form is DIRECT-mode
    /// only, so from a running program it is always Can't-use-in-program (errnum 44); the
    /// **read** form (0 input args, 1 `OUT`) pushes the current project name and is allowed in
    /// a program. Any other shape is Illegal function call (4). (`PROJECT=v` is an ordinary
    /// variable assignment handled by the compiler, never reaching here.)
    fn file_project(
        &mut self,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<(), RuntimeError> {
        use crate::builtins::files::ERR_CANT_USE_IN_PROGRAM;
        use crate::builtins::ERR_ILLEGAL_FUNCTION_CALL;
        if args.is_empty() && out_argc == 1 && !wants_value {
            let name: SbStr = self.current_project.encode_utf16().collect();
            self.stack.push(Value::Str(name));
            return Ok(());
        }
        if args.len() == 1 && out_argc == 0 && !wants_value {
            return Err(RuntimeError::new(ERR_CANT_USE_IN_PROGRAM));
        }
        Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL))
    }

    /// Route the source-edit family (M6-T4: `PRGEDIT`/`PRGGET$`/`PRGSET`/`PRGINS`/`PRGDEL`/
    /// `PRGNAME$`/`PRGSIZE`). Returns `Ok(true)` if `name` is a PRG command (and was handled,
    /// pushing any function value), `Ok(false)` to fall through to the stateless dispatch.
    fn call_prg(&mut self, name: &str, args: &[Value], wants_value: bool) -> Result<bool, VmError> {
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "PRGEDIT" => self.prg_edit_cmd(&args).map_err(sb)?,
            "PRGGET$" => {
                let s = self.prg_get(&args).map_err(sb)?;
                if wants_value {
                    self.stack.push(Value::Str(s));
                }
            }
            "PRGSET" => self.prg_set(&args).map_err(sb)?,
            "PRGINS" => self.prg_ins(&args).map_err(sb)?,
            "PRGDEL" => self.prg_del(&args).map_err(sb)?,
            "PRGNAME$" => {
                let s = self.prg_name(&args).map_err(sb)?;
                if wants_value {
                    self.stack.push(Value::Str(s));
                }
            }
            "PRGSIZE" => {
                let n = self.prg_size(&args).map_err(sb)?;
                if wants_value {
                    self.stack.push(Value::Int(n));
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Validate `slot` (0..3 else errnum 10) and reject the currently-running slot (errnum 4)
    /// — the running-slot guard shared by `PRGEDIT` (you cannot edit the slot you are
    /// executing). Returns the slot as a `usize` index.
    fn prg_validate_edit_slot(&self, slot: i32) -> Result<usize, RuntimeError> {
        if !(0..=3).contains(&slot) {
            return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
        }
        if slot as u8 == self.current_slot {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        Ok(slot as usize)
    }

    /// `PRGEDIT slot [,line]` — select the edit target. Arg count must be 1 or 2 (else errnum
    /// 4); the slot is range-checked to 0..3 and may not be the running slot. With one
    /// argument the current line is the first line (0); with two, the second argument is the
    /// current line, where `-1` selects the last line and a value past the program → errnum 10.
    fn prg_edit_cmd(&mut self, args: &[Value]) -> Result<(), RuntimeError> {
        if args.is_empty() || args.len() > 2 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let slot = self.prg_validate_edit_slot(args[0].to_int()?)?;
        let len = self.prg_slots[slot].lines.len();
        let line = match args.get(1) {
            None => 0usize,
            Some(v) => {
                let l = v.to_int()?;
                if l == -1 {
                    // Last line; an empty slot has no lines, so clamp to 0.
                    len.saturating_sub(1)
                } else if l < 0 || (l as usize) > len {
                    return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
                } else {
                    l as usize
                }
            }
        };
        self.prg_edit = Some((slot as u8, line));
        Ok(())
    }

    /// Resolve the active `(slot, line)` edit target, or errnum 38 (`Use PRGEDIT before any
    /// PRG function`) when none is set — the cold-state guard the four current-line mutators
    /// share, checked before their argument count.
    fn prg_target(&self) -> Result<(usize, usize), RuntimeError> {
        match self.prg_edit {
            Some((s, l)) => Ok((s as usize, l)),
            None => Err(RuntimeError::new(ERR_USE_PRGEDIT)),
        }
    }

    /// `PRGGET$()` — the current line's source text (LF terminator already stripped), or the
    /// empty string when the current line is past the end of the program. Requires an active
    /// PRGEDIT target (errnum 38, checked first); any argument → errnum 4.
    fn prg_get(&self, args: &[Value]) -> Result<SbStr, RuntimeError> {
        let (slot, line) = self.prg_target()?;
        if !args.is_empty() {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        Ok(self.prg_slots[slot]
            .lines
            .get(line)
            .cloned()
            .unwrap_or_default())
    }

    /// `PRGSET str$` — replace the current line with `str$`. A string containing `CHR$(10)`
    /// writes multiple lines; when the current line is past the end (PRGGET$ would be empty)
    /// the line(s) are appended instead. Requires PRGEDIT (errnum 38, first); exactly one
    /// string argument, else errnum 4.
    fn prg_set(&mut self, args: &[Value]) -> Result<(), RuntimeError> {
        let (slot, line) = self.prg_target()?;
        if args.len() != 1 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        // A non-string operand is an Illegal function call (4), not a Type mismatch (8).
        let text = args[0]
            .as_str()
            .map_err(|_| RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL))?;
        let segs = crate::builtins::prg::split_separated(text);
        let lines = &mut self.prg_slots[slot].lines;
        if line >= lines.len() {
            lines.extend(segs);
        } else {
            lines.splice(line..line + 1, segs);
        }
        Ok(())
    }

    /// `PRGINS str$ [,flag]` — insert line(s) at the current line: flag 0/omitted before it,
    /// flag 1 after it. A `CHR$(10)` in `str$` inserts multiple lines; an empty string inserts
    /// one blank line. Requires PRGEDIT (errnum 38, first); 1 or 2 arguments with a string
    /// first operand, else errnum 4.
    fn prg_ins(&mut self, args: &[Value]) -> Result<(), RuntimeError> {
        let (slot, line) = self.prg_target()?;
        if args.is_empty() || args.len() > 2 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let text = args[0]
            .as_str()
            .map_err(|_| RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL))?;
        let after = match args.get(1) {
            Some(v) => v.to_int()? == 1,
            None => false,
        };
        let segs = crate::builtins::prg::split_separated(text);
        let lines = &mut self.prg_slots[slot].lines;
        let at = if after {
            (line + 1).min(lines.len())
        } else {
            line.min(lines.len())
        };
        lines.splice(at..at, segs);
        Ok(())
    }

    /// `PRGDEL [count]` — delete `count` lines from the current line (default 1). A negative
    /// count deletes all remaining lines; a count of 0, or a positive count past the remaining
    /// lines, → errnum 10. Requires PRGEDIT (errnum 38, first); more than one argument → 4.
    fn prg_del(&mut self, args: &[Value]) -> Result<(), RuntimeError> {
        let (slot, line) = self.prg_target()?;
        if args.len() > 1 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let count = match args.first() {
            Some(v) => v.to_int()?,
            None => 1,
        };
        if count == 0 {
            return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
        }
        let lines = &mut self.prg_slots[slot].lines;
        let remaining = lines.len().saturating_sub(line);
        if count < 0 {
            // Delete all remaining lines from the current line down.
            lines.truncate(line.min(lines.len()));
        } else {
            let n = count as usize;
            if n > remaining {
                return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
            }
            lines.drain(line..line + n);
        }
        Ok(())
    }

    /// `PRGNAME$([slot])` — the file name last handled by LOAD/SAVE for a slot. No argument →
    /// the running slot; one argument is range-checked 0..3 (errnum 10). 2+ args → errnum 4.
    fn prg_name(&self, args: &[Value]) -> Result<SbStr, RuntimeError> {
        if args.len() > 1 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let slot = match args.first() {
            None => self.current_slot as i32,
            Some(v) => v.to_int()?,
        };
        if !(0..=3).contains(&slot) {
            return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
        }
        Ok(self.prg_slots[slot as usize].name.clone())
    }

    /// `PRGSIZE([slot[,type]])` — a slot's size: type 0 lines (default), 1 characters, 2 free
    /// characters. No argument → the running slot. Slot is range-checked 0..3 and type 0..2
    /// (errnum 10); 3+ args → errnum 4.
    fn prg_size(&self, args: &[Value]) -> Result<i32, RuntimeError> {
        if args.len() > 2 {
            return Err(RuntimeError::new(ERR_ILLEGAL_FUNCTION_CALL));
        }
        let slot = match args.first() {
            None => self.current_slot as i32,
            Some(v) => v.to_int()?,
        };
        if !(0..=3).contains(&slot) {
            return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
        }
        let ty = match args.get(1) {
            None => 0,
            Some(v) => v.to_int()?,
        };
        if !(0..=2).contains(&ty) {
            return Err(RuntimeError::new(ERR_OUT_OF_RANGE));
        }
        let s = &self.prg_slots[slot as usize];
        let n = match ty {
            0 => s.lines.len(),
            1 => s.char_count(),
            _ => s.free_count(),
        };
        Ok(n as i32)
    }

    /// Handle the RNG builtins against the VM-owned [`Rng`](crate::rng::Rng). Returns
    /// `Ok(Some(value))` for `RND`/`RNDF` (the drawn value), `Ok(Some(Void))` for the
    /// `RANDOMIZE` statement, or `Ok(None)` when `name` is not an RNG builtin (the caller
    /// then falls through to the stateless dispatch). Argument validation follows the
    /// `spec/instructions/{rnd,rndf,randomize}.yaml` contract: bad arg count → Illegal
    /// function call (4), string arg → Type mismatch (8), out-of-range seed/max → Out of
    /// range (10).
    fn call_rng(&mut self, name: &str, args: &[Value]) -> Result<Option<Value>, RuntimeError> {
        match name {
            "RND" => {
                // RND(max) draws from series 0; RND(seed_id, max) selects the series.
                let (seed_id, max) = match args {
                    [m] => (0, m.to_int()?),
                    [s, m] => (rng_seed_id(s)?, m.to_int()?),
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                if max < 0 {
                    return Err(crate::builtins::out_of_range());
                }
                Ok(Some(Value::Int(self.rng.rnd(seed_id, max))))
            }
            "RNDF" => {
                // RNDF() draws from series 0; RNDF(seed_id) selects the series.
                let seed_id = match args {
                    [] => 0,
                    [s] => rng_seed_id(s)?,
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                Ok(Some(Value::Real(self.rng.rndf(seed_id))))
            }
            "RANDOMIZE" => {
                // RANDOMIZE seed_id [, seed_value]; seed_value 0/omitted → entropy.
                let (seed_id, seed_value) = match args {
                    [s] => (rng_seed_id(s)?, 0),
                    [s, v] => (rng_seed_id(s)?, v.to_int()?),
                    _ => {
                        return Err(RuntimeError::new(
                            crate::builtins::ERR_ILLEGAL_FUNCTION_CALL,
                        ))
                    }
                };
                self.rng.randomize(seed_id, seed_value);
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    /// Frame-timing builtins `WAIT`/`VSYNC` (M4-T3) over the VM-owned [`FrameClock`].
    /// Both are statements (function use → errnum 4) taking an optional integer frame count
    /// that defaults to 1, with negative counts treated as 0 ("0: Ignore"). WAIT counts from
    /// the present frame; VSYNC counts from the previous VSYNC (see `clock::FrameClock`).
    /// Headless the wait resolves instantly, advancing `MAINCNT` by the resolved count; the
    /// per-frame background machinery (sprite/BG animation) steps once per elapsed frame.
    fn call_timing(
        &mut self,
        name: &str,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        use crate::builtins::illegal;
        let count = match name {
            "WAIT" | "VSYNC" => {
                if wants_value {
                    return Err(illegal());
                }
                match args.len() {
                    0 => 1,
                    1 => {
                        let n = args[0].to_int()?;
                        if n < 0 {
                            0
                        } else {
                            n as u64
                        }
                    }
                    _ => return Err(illegal()),
                }
            }
            _ => return Ok(None),
        };
        if self.interactive {
            // Host-driven VBlank model: arm a pending target; the host's tick_frame calls
            // advance the counter one frame at a time and resolve_wait clears it when reached.
            // Animations advance per-frame through tick_frame during the wait, not in a burst.
            match name {
                "WAIT" => self.clock.begin_wait(count),
                "VSYNC" => self.clock.begin_vsync(count),
                _ => {}
            }
            // count == 0 ("0: Ignore") does not yield (begin_* handled it inline).
            if count > 0 {
                self.yielded = true;
            }
        } else {
            // Headless instant-jump model (used by run() and all deterministic tests).
            let elapsed = match name {
                "WAIT" => self.clock.wait(count),
                "VSYNC" => self.clock.vsync(count),
                _ => 0,
            };
            // Sprite (M3-T2) and BG (M3-T5) animations advance one step per displayed frame.
            for sp in &mut self.sprites {
                sp.tick(elapsed);
            }
            for bg in &mut self.bg {
                bg.tick(elapsed);
            }
            // The screen fader advances one frame per elapsed frame in headless WAIT/VSYNC.
            self.fader.tick(elapsed);
            // Frame-timing builtins are natural yield points: ask the host to return to the
            // platform loop so live input (BUTTON polls) and the wall-clock frame pacer can run.
            // WAIT/VSYNC count <= 0 is documented as "Ignore" and should not yield.
            if count > 0 {
                self.yielded = true;
            }
        }
        Ok(Some(Value::Void))
    }

    /// Route a console builtin (M1-T8) over the VM-owned [`Console`] / screen state.
    /// Returns `Ok(Some(value))` when handled (statement commands return [`Value::Void`]),
    /// or `Ok(None)` when `name` is not a console builtin (the caller falls through to the
    /// stateless dispatch). Argument validation follows the console specs.
    fn call_console(
        &mut self,
        name: &str,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        use crate::builtins::console as cons;
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "LOCATE" => {
                cons::locate(self.active_console_mut(), &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "COLOR" => {
                cons::color(self.active_console_mut(), &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "CLS" => {
                cons::cls(self.active_console_mut(), &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "ACLS" => {
                // Validate the argument shape against the console builtin, then replace BOTH
                // physical screens' consoles with fresh default grids (ACLS is a system reset).
                let mut dummy = Console::top();
                cons::acls(&mut dummy, &args, wants_value)?;
                self.consoles = [Console::top(), Console::bottom()];
                // ACLS resets the wider screen draw state too (not just the console grid),
                // all hw_verified via reset-proofs (`<set>:ACLS:<get>()`, sb-oracle, acls.yaml):
                //   BACKCOLOR -> 0   (`BACKCOLOR &HFF112233:ACLS:?BACKCOLOR()` -> 0)
                //   GCOLOR    -> &HFFFFFFFF / -1 default white (`GCOLOR RGB(1,2,3):ACLS:?GCOLOR()` -> -1)
                //   DISPLAY   -> 0   (`XSCREEN 2:DISPLAY 1:ACLS:DISPLAY()` -> 0; the documented
                //                     XSCREEN 0 / DISPLAY / VISIBLE 1,1,1,1 reset = ScreenConfig::new)
                // TABSTEP is NOT reset (`TABSTEP=8:ACLS:TABSTEP` -> 8, stays a sysvar), so we
                // deliberately leave self.tabstep alone — the old `tabstep = 4` here was wrong.
                self.back_color = 0;
                self.fader = Fader::new();
                self.grp.color = 0xFFFF_FFFF;
                // ACLS reloads the firmware defaults (the documented "LOAD DEFSP/DEFBG" + font/
                // SPDEF reset, acls.yaml): GRP4 ← sprite sheet, GRP5 ← BG sheet, GRPF ← font, and
                // the SPDEF definition-template table back to its built-in state.
                self.grp.reload_defaults();
                // The SPDEF definition table is shared (global) across both screens — reset it
                // once. (ACLS reloads default sheets but does NOT clear live sprite slots, so the
                // per-screen sprite tables are left as-is, preserving the prior behavior.)
                self.spdef.reset();
                self.screen = ScreenConfig::new();
                // DISPLAY resets to 0 (Upper), so the GRP command-target screen must follow —
                // otherwise `DISPLAY 1:ACLS` would leave subsequent draws routed to the Touch
                // screen while DISPLAY() reads back 0.
                self.grp.active = 0;
                Ok(Some(Value::Void))
            }
            "INKEY$" => Ok(Some(cons::inkey(&mut self.input, &args)?)),
            "CHKCHR" => Ok(Some(cons::chkchr(
                self.active_console(),
                &args,
                wants_value,
            )?)),
            "BACKCOLOR" => Ok(Some(self.backcolor(&args, wants_value)?)),
            "FADE" => Ok(Some(self.fade(&args, wants_value)?)),
            "FADECHK" => Ok(Some(self.fadechk(&args, wants_value)?)),
            "ATTR" => {
                cons::attr(self.active_console_mut(), &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "SCROLL" => {
                cons::scroll(self.active_console_mut(), &args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "WIDTH" => Ok(Some(cons::width(
                self.active_console_mut(),
                &args,
                wants_value,
            )?)),
            "FONTDEF" => {
                // FONTDEF edits are global to the console font: apply to the active screen, then
                // mirror the custom glyph table to the other screen.
                cons::fontdef(self.active_console_mut(), &args, wants_value)?;
                let active = self.active_screen();
                let other = 1 - active;
                let glyphs = self.consoles[active].custom_glyphs().clone();
                self.consoles[other].set_custom_glyphs(glyphs);
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    /// Route a screen-configuration builtin (XSCREEN/DISPLAY/VISIBLE/HARDWARE, M4-T4) over
    /// the VM-owned [`ScreenConfig`]. Returns `Ok(Some(value))` when handled (the statement
    /// commands return [`Value::Void`]; DISPLAY's GET form and HARDWARE return an Integer),
    /// or `Ok(None)` when `name` is not a screen builtin (the caller falls through to the
    /// stateless dispatch). Argument/range validation follows the disassembled handlers.
    fn call_screen(
        &mut self,
        name: &str,
        args: &[Value],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "XSCREEN" => {
                self.screen.xscreen(&args, wants_value)?;
                let (w, h) = self.screen.display_size();
                self.grp.set_display_area(w, h);
                // Keep the active console's grid dimensions in sync with the new resolution.
                self.resize_active_console(w as usize, h as usize);
                Ok(Some(Value::Void))
            }
            "DISPLAY" => {
                let result = self.screen.display(&args, wants_value)?;
                // The SET form (no return value) selects the target screen: route subsequent
                // GRP commands there, then refresh that screen's display area/clip.
                if result.is_none() {
                    self.grp.active = self.screen.display as usize;
                    let (w, h) = self.screen.display_size();
                    self.grp.set_display_area(w, h);
                    // The newly selected screen's console must match its resolution.
                    self.resize_active_console(w as usize, h as usize);
                }
                Ok(Some(result.unwrap_or(Value::Void)))
            }
            "VISIBLE" => {
                self.screen.visible(&args, wants_value)?;
                Ok(Some(Value::Void))
            }
            "HARDWARE" => Ok(Some(self.screen.hardware(&args)?)),
            _ => Ok(None),
        }
    }

    /// Route a BGM command (BGMPLAY/BGMSTOP/BGMCHK/BGMVAR/BGMVOL/BGMSET/BGMSETD/BGMCLEAR,
    /// M5-T3) over the VM-owned [`AudioState`]. Returns `Ok(true)` when handled — the
    /// function/read forms (`BGMCHK`, the 2-arg `BGMVAR`) push their result onto the stack —
    /// or `Ok(false)` when `name` is not a BGM command (the caller falls through to the
    /// stateless dispatch). The SET/GET-style form selection follows the disassembled
    /// handlers' result-count (`wants_value`) + argument-count checks.
    fn call_sound(
        &mut self,
        name: &str,
        args: &[Value],
        wants_value: bool,
    ) -> Result<bool, VmError> {
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            "BGMPLAY" => self.audio.bgmplay(&args, wants_value).map_err(sb)?,
            "BGMSTOP" => self.audio.bgmstop(&args, wants_value).map_err(sb)?,
            "BGMVOL" => self.audio.bgmvol(&args, wants_value).map_err(sb)?,
            "BGMSET" => self.audio.bgmset(&args, wants_value).map_err(sb)?,
            "BGMCLEAR" => self.audio.bgmclear(&args, wants_value).map_err(sb)?,
            "BGMCHK" => {
                let v = self.audio.bgmchk(&args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "BGMVAR" => {
                if let Some(v) = self.audio.bgmvar(&args, wants_value).map_err(sb)? {
                    if wants_value {
                        self.stack.push(v);
                    }
                }
            }
            "BGMSETD" => self.call_bgmsetd(&args, wants_value)?,
            // SFX / voice (M5-T4): preset sound effects, synthesized speech, the music
            // effector and user MML instruments, all over `AudioState`.
            "BEEP" => self.audio.beep(&args, wants_value).map_err(sb)?,
            "TALK" => self.audio.talk(&args, wants_value).map_err(sb)?,
            "TALKSTOP" => self.audio.talkstop(&args, wants_value).map_err(sb)?,
            "TALKCHK" => {
                let v = self.audio.talkchk(&args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "EFCSET" => self.audio.efcset(&args, wants_value).map_err(sb)?,
            "EFCON" => self.audio.efcon(&args, wants_value).map_err(sb)?,
            "EFCOFF" => self.audio.efcoff(&args, wants_value).map_err(sb)?,
            "EFCWET" => self.audio.efcwet(&args, wants_value).map_err(sb)?,
            "WAVSET" => self.audio.wavset(&args, wants_value).map_err(sb)?,
            "WAVSETA" => self.audio.wavseta(&args, wants_value).map_err(sb)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `BGMSETD tune, "@Label"` — register a user-defined tune (128..255) from MML stored in
    /// a `DATA` block. Statement (return context → errnum 4); exactly 2 args (else errnum 4);
    /// tune outside 128..255 → errnum 10; a non-string label → errnum 8; an undefined label →
    /// errnum 14 (the RESTORE-shared lookup); MML that fails to compile → errnum 47. The MML
    /// is gathered from consecutive string `DATA` items under the label, terminated by the
    /// first numeric `DATA` item (`bgmsetd.yaml`). Lives in the VM (not `AudioState`) because
    /// it reads the program's DATA pool.
    fn call_bgmsetd(&mut self, args: &[Value], wants_value: bool) -> Result<(), VmError> {
        use crate::builtins::sound::{compile_mml, ranged};
        if wants_value || args.len() != 2 {
            return Err(sb(crate::builtins::illegal()));
        }
        let tune = ranged(&args[0], 128, 255).map_err(sb)?;
        let label = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(sb(crate::builtins::type_mismatch())),
        };
        // A label may carry a leading `N:` MML-channel prefix (corpus `"1:@MML"`); strip it,
        // then the `@`, and case-fold like the other label lookups.
        let raw = String::from_utf16_lossy(&label);
        let after_chan = raw.rsplit(':').next().unwrap_or(&raw);
        let name = after_chan.trim_start_matches('@').to_ascii_uppercase();
        // A missing label is NOT an error (hw_verified: real SB 3.6.0 returns NOERR for
        // `BGMSETD 128,"@NOPE"` with no matching DATA block — M7-T2): the label lookup yields
        // no MML, an empty tune is compiled and registered. Only a present label gathers DATA.
        let idx = self
            .program
            .data_labels
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, i)| *i);
        // Gather consecutive string DATA items, stopping at the first numeric item (the
        // documented terminator) or the end of the pool.
        let mut mml = String::new();
        if let Some(mut k) = idx {
            while let Some(Const::Str(s)) = self.program.data.get(k) {
                mml.push_str(&String::from_utf16_lossy(s));
                k += 1;
            }
        }
        let song = compile_mml(&mml).map_err(sb)?;
        self.audio.register_tune(tune, song);
        Ok(())
    }

    /// Route a graphics builtin (M2-T1) over the VM-owned [`GrpState`]. Returns `Ok(true)`
    /// when handled (the function's result values are pushed onto the stack in source
    /// order — none for a plain command, one for a function, several for an `OUT` form), or
    /// `Ok(false)` when `name` is not a graphics builtin (the caller falls through to the
    /// stateless dispatch). The SET/GET form is chosen by the **return count**, which
    /// collapses the function (`wants_value`) and `OUT` (`out_argc`) spellings — exactly the
    /// disassembled handlers' `[r0,#0xc]` check.
    fn call_graphics(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        use crate::builtins::graphics as gfx;
        let ret_count = if wants_value { 1 } else { out_argc as usize };
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        let results = match name {
            "RGB" => gfx::rgb(&args, ret_count),
            "RGBREAD" => gfx::rgbread(&args, ret_count),
            "GSPOIT" => gfx::gspoit(&self.grp, &args, ret_count),
            "GPAGE" => gfx::gpage(&mut self.grp, &args, ret_count),
            "GCLS" => gfx::gcls(&mut self.grp, &args, ret_count),
            "GCOLOR" => gfx::gcolor(&mut self.grp, &args, ret_count),
            "GPRIO" => gfx::gprio(&mut self.grp, &args, ret_count),
            "GCLIP" => gfx::gclip(&mut self.grp, &args, ret_count),
            "GPSET" => gfx::gpset(&mut self.grp, &args, ret_count),
            "GLINE" => gfx::gline(&mut self.grp, &args, ret_count),
            "GBOX" => gfx::gbox(&mut self.grp, &args, ret_count),
            "GFILL" => gfx::gfill(&mut self.grp, &args, ret_count),
            "GCIRCLE" => gfx::gcircle(&mut self.grp, &args, ret_count),
            "GTRI" => gfx::gtri(&mut self.grp, &args, ret_count),
            "GPAINT" => gfx::gpaint(&mut self.grp, &args, ret_count),
            "GCOPY" => gfx::gcopy(&mut self.grp, &args, ret_count),
            "GSAVE" => gfx::gsave(&mut self.grp, &args, ret_count),
            "GLOAD" => gfx::gload(&mut self.grp, &args, ret_count),
            _ => return Ok(false),
        };
        for v in results.map_err(sb)? {
            self.stack.push(v);
        }
        Ok(true)
    }

    /// Route a sprite lifecycle builtin (M3-T1) over the VM-owned [`SpriteState`]. Returns
    /// `Ok(true)` when handled (pushing the command's result values — none for `SPSET`
    /// (explicit form) / `SPCLR` / `SPSHOW` / `SPHIDE`, one for an `SPSET` auto-allocate or
    /// `SPUSED`), or `Ok(false)` when `name` is not a sprite builtin. Like the graphics
    /// commands, the SET/GET-style form is chosen by the **return count**, collapsing the
    /// function (`wants_value`) and `OUT` (`out_argc`) spellings.
    fn call_sprite(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        use crate::builtins::sprite as spr;
        let ret_count = if wants_value { 1 } else { out_argc as usize };
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        // SPANIM and SPFUNC need the program (DATA pool / @label resolution), so the VM
        // orchestrates them; the rest are pure over the sprite table.
        if name == "SPANIM" {
            self.do_spanim(&args, ret_count)?;
            return Ok(true);
        }
        if name == "SPFUNC" {
            self.do_spfunc(&args, ret_count)?;
            return Ok(true);
        }
        // SPDEF's bulk forms read a numeric array / DATA `@label`, so the VM orchestrates it
        // (the scalar define/copy/reset/getter forms stay pure over the template table).
        if name == "SPDEF" {
            self.do_spdef(&args, ret_count)?;
            return Ok(true);
        }
        // Every sprite builtin operates on the ACTIVE DISPLAY screen's table (#83). Compute the
        // index into a local so the per-call `&mut self.sprites[scr]` borrow stays disjoint from
        // the shared `&self.spdef` borrow SPSET/SPCHR also need.
        let scr = self.active_screen();
        let results = match name {
            "SPSET" => spr::spset(&mut self.sprites[scr], &self.spdef, &args, ret_count),
            "SPCLR" => spr::spclr(&mut self.sprites[scr], &args, ret_count),
            "SPSHOW" => spr::spshow(&mut self.sprites[scr], &args, ret_count),
            "SPHIDE" => spr::sphide(&mut self.sprites[scr], &args, ret_count),
            "SPUSED" => spr::spused(&self.sprites[scr], &args, ret_count),
            "SPVAR" => spr::spvar(&mut self.sprites[scr], &args, ret_count),
            "SPSTART" => spr::spstart(&mut self.sprites[scr], &args, ret_count),
            "SPSTOP" => spr::spstop(&mut self.sprites[scr], &args, ret_count),
            "SPLINK" => spr::splink(&mut self.sprites[scr], &args, ret_count),
            "SPUNLINK" => spr::spunlink(&mut self.sprites[scr], &args, ret_count),
            "SPOFS" => spr::spofs(&mut self.sprites[scr], &args, ret_count),
            "SPSCALE" => spr::spscale(&mut self.sprites[scr], &args, ret_count),
            "SPROT" => spr::sprot(&mut self.sprites[scr], &args, ret_count),
            "SPCOLOR" => spr::spcolor(&mut self.sprites[scr], &args, ret_count),
            "SPHOME" => spr::sphome(&mut self.sprites[scr], &args, ret_count),
            "SPCHR" => spr::spchr(&mut self.sprites[scr], &self.spdef, &args, ret_count),
            "SPPAGE" => spr::sppage(&mut self.sprites[scr], &args, ret_count),
            "SPCLIP" => spr::spclip(
                &mut self.sprites[scr],
                &args,
                ret_count,
                self.screen.display_size(),
            ),
            "SPCOL" => spr::spcol(&mut self.sprites[scr], &args, ret_count),
            "SPCOLVEC" => spr::spcolvec(&mut self.sprites[scr], &args, ret_count),
            "SPCHK" => spr::spchk(&self.sprites[scr], &args, ret_count),
            "SPHITSP" => spr::sphitsp(&mut self.sprites[scr], &args, ret_count),
            "SPHITRC" => spr::sphitrc(&mut self.sprites[scr], &args, ret_count),
            "SPHITINFO" => spr::sphitinfo(&self.sprites[scr], &args, ret_count),
            _ => return Ok(false),
        };
        for v in results.map_err(sb)? {
            self.stack.push(v);
        }
        Ok(true)
    }

    /// Route a BG core builtin (M3-T4) over the VM-owned [`BgState`]. Returns `Ok(true)` when
    /// handled (pushing the command's result values — none for the SET-form statements, one
    /// for `BGGET`/`BGPAGE`/`BGROT`/`BGCOLOR` GET, two/three for the `BGOFS`/`BGSCALE`/
    /// `BGHOME` OUT forms), or `Ok(false)` when `name` is not a BG builtin. Like the sprite
    /// commands, the SET/GET-style form is chosen by the **return count**, collapsing the
    /// function (`wants_value`) and `OUT` (`out_argc`) spellings.
    fn call_bg(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        use crate::builtins::bg as b;
        let ret_count = if wants_value { 1 } else { out_argc as usize };
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        // BGANIM and BGFUNC need the program (DATA pool / @label resolution), so the VM
        // orchestrates them; the rest are pure over the BG state.
        if name == "BGANIM" {
            self.do_bganim(&args, ret_count)?;
            return Ok(true);
        }
        if name == "BGFUNC" {
            self.do_bgfunc(&args, ret_count)?;
            return Ok(true);
        }
        // Every BG builtin operates on the ACTIVE DISPLAY screen's table (#84).
        let scr = self.active_screen();
        let results = match name {
            "BGSCREEN" => b::bgscreen(&mut self.bg[scr], &args, ret_count),
            "BGPAGE" => b::bgpage(&mut self.bg[scr], &args, ret_count),
            "BGPUT" => b::bgput(&mut self.bg[scr], &args, ret_count),
            "BGGET" => b::bgget(&self.bg[scr], &args, ret_count),
            "BGFILL" => b::bgfill(&mut self.bg[scr], &args, ret_count),
            "BGCLR" => b::bgclr(&mut self.bg[scr], &args, ret_count),
            "BGOFS" => b::bgofs(&mut self.bg[scr], &args, ret_count),
            "BGROT" => b::bgrot(&mut self.bg[scr], &args, ret_count),
            "BGSCALE" => b::bgscale(&mut self.bg[scr], &args, ret_count),
            "BGCOLOR" => b::bgcolor(&mut self.bg[scr], &args, ret_count),
            "BGSHOW" => b::bgshow(&mut self.bg[scr], &args, ret_count),
            "BGHIDE" => b::bghide(&mut self.bg[scr], &args, ret_count),
            "BGHOME" => b::bghome(&mut self.bg[scr], &args, ret_count),
            "BGCLIP" => b::bgclip(&mut self.bg[scr], &args, ret_count),
            "BGVAR" => b::bgvar(&mut self.bg[scr], &args, ret_count),
            "BGCHK" => b::bgchk(&self.bg[scr], &args, ret_count),
            "BGSTART" => b::bgstart(&mut self.bg[scr], &args, ret_count),
            "BGSTOP" => b::bgstop(&mut self.bg[scr], &args, ret_count),
            "BGCOPY" => b::bgcopy(&mut self.bg[scr], &args, ret_count),
            "BGCOORD" => b::bgcoord(&self.bg[scr], &args, ret_count),
            "BGSAVE" => b::bgsave(&self.bg[scr], &args, ret_count),
            "BGLOAD" => b::bgload(&mut self.bg[scr], &args, ret_count),
            _ => return Ok(false),
        };
        for v in results.map_err(sb)? {
            self.stack.push(v);
        }
        Ok(true)
    }

    /// Route a hardware-input builtin (M4-T1) over the VM-owned [`InputState`]. Returns
    /// `Ok(true)` when handled — `BUTTON` pushes its one bitmask result, `STICK`/`STICKEX`
    /// push two OUT axis Doubles, `BREPEAT` pushes nothing — or `Ok(false)` when `name` is
    /// not an input builtin. Like the graphics/sprite/BG commands, the function
    /// (`wants_value`) and `OUT` (`out_argc`) spellings collapse into one `ret_count`.
    fn call_input(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        use crate::builtins::input as inp;
        let ret_count = if wants_value { 1 } else { out_argc as usize };
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        let results = match name {
            "BUTTON" => inp::button(&self.input, &args, ret_count),
            "STICK" => inp::stick(&self.input, &args, ret_count),
            "STICKEX" => inp::stickex(&self.input, &args, ret_count),
            "BREPEAT" => inp::brepeat(&mut self.input, &args, ret_count),
            "TOUCH" => inp::touch(&self.input, &args, ret_count),
            "KEY" => inp::key(&mut self.input, &args, ret_count),
            _ => return Ok(false),
        };
        for v in results.map_err(sb)? {
            self.stack.push(v);
        }
        Ok(true)
    }

    /// Route a faithful "limitation stub" builtin (M6-T5): the `XON`/`XOFF` feature gate, the
    /// microphone (`MICSTART`/`MICSTOP`/`MICDATA`/`MICSAVE`), the motion sensors (`GYROA`/
    /// `GYROV`/`GYROSYNC`/`ACCEL`), wireless multiplayer (`MPSTART`/`MPEND`/`MPSET`/`MPSTAT`/
    /// `MPSEND`/`MPRECV`/`MPGET`/`MPNAME$`), and `DIALOG`. Returns `Ok(true)` when handled
    /// (pushing the function/OUT results itself), `Ok(false)` otherwise (the caller falls
    /// through to the stateless dispatch). None of the underlying hardware exists headless, so
    /// each reproduces its disassembled arg-shape / range / type guards and the XON-MIC /
    /// XON-MOTION availability errors (36/37) rather than the device. The MP commands run as if
    /// wireless is reachable (the `@0x305612` restriction flag is 0 in DIRECT/program mode) but
    /// with no peers, so peer-indexed reads are out of range and `MPRECV` yields no data.
    fn call_device(
        &mut self,
        name: &str,
        args: &[Value],
        out_argc: u8,
        wants_value: bool,
    ) -> Result<bool, VmError> {
        use crate::builtins::device as dev;
        let ret_count = if wants_value { 1 } else { out_argc as usize };
        let args: Vec<Value> = args.iter().map(|v| v.deref()).collect();
        match name {
            // XON/XOFF: the parser emits a synthetic feature-code operand (0=MOTION, 1=EXPAD,
            // 2=MIC). Enabling EXPAD sets RESULT TRUE (xon.yaml).
            "XON" | "XOFF" => {
                let code = args.first().map(Value::to_int).transpose().map_err(sb)?;
                let feature = code
                    .and_then(dev::Feature::from_code)
                    .ok_or_else(|| sb(crate::builtins::syntax_error()))?;
                let on = name == "XON";
                self.device.set(feature, on);
                if on && feature == dev::Feature::Expad {
                    self.result = 1; // XON EXPAD -> RESULT TRUE
                }
            }
            "MICSTART" => dev::micstart(&self.device, &args, wants_value).map_err(sb)?,
            "MICSTOP" => dev::micstop(&self.device, &args, wants_value).map_err(sb)?,
            "MICDATA" => {
                let v = dev::micdata(&self.device, &args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "MICSAVE" => dev::micsave(&args, wants_value).map_err(sb)?,
            "GYROA" | "GYROV" | "ACCEL" => {
                for v in dev::motion_read(&self.device, &args, ret_count).map_err(sb)? {
                    self.stack.push(v);
                }
            }
            "GYROSYNC" => dev::gyrosync(&self.device, &args, ret_count).map_err(sb)?,
            "MPSTART" => {
                dev::mpstart(&args, wants_value).map_err(sb)?;
                self.result = 0; // offline: no session established -> RESULT FALSE
            }
            "MPEND" => dev::mpend(&args, wants_value).map_err(sb)?,
            "MPSET" => dev::mpset(&args, wants_value).map_err(sb)?,
            "MPSTAT" => {
                let v = dev::mpstat(&args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "MPSEND" => dev::mpsend(&args, wants_value).map_err(sb)?,
            "MPRECV" => {
                for v in dev::mprecv(&args, ret_count).map_err(sb)? {
                    self.stack.push(v);
                }
            }
            "MPGET" => {
                let v = dev::mpget(&args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "MPNAME$" => {
                let v = dev::mpname(&args, wants_value).map_err(sb)?;
                if wants_value {
                    self.stack.push(v);
                }
            }
            "DIALOG" => {
                let outcome = dev::dialog(&args, wants_value).map_err(sb)?;
                self.result = outcome.result;
                if let Some(v) = outcome.push {
                    if wants_value {
                        self.stack.push(v);
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `BGANIM layer, target, data[, loop]` — define and start a BG-layer animation. Mirrors
    /// [`Vm::do_spanim`]: the third operand selects the form — a numeric **array** (form 1),
    /// an `"@label"` **string** pointing at DATA (form 2), or an inline numeric **argument
    /// list** (form 3). After the shared argcount>=3 / return-count==0 gate (errnum 4), the
    /// data is flattened to a `Time,Item[,Item],…` list and handed to [`BgState::set_anim`].
    /// BG has no UV/I channel (errnum 4 via [`bg::parse_bg_target`]).
    fn do_bganim(&mut self, args: &[Value], ret_count: usize) -> Result<(), VmError> {
        use crate::builtins::bg;
        use crate::builtins::data::read_values;
        use crate::builtins::sprite as spr;
        // Gate: a return value, or fewer than 3 arguments, is Illegal function call (4).
        if ret_count != 0 || args.len() < 3 {
            return Err(sb(crate::builtins::illegal()));
        }
        let layer = {
            let i = args[0].to_int().map_err(sb)?;
            if !sb_render::bg::BgState::in_range(i) {
                return Err(sb(crate::builtins::out_of_range()));
            }
            i as usize
        };
        let (channel, relative) = bg::parse_bg_target(&args[1]).map_err(sb)?;
        let stride = 1 + spr::anim_items(channel);

        let (data, loop_count): (Vec<f64>, i32) = match &args[2] {
            // Form 1 — keyframes in a numeric array; an explicit trailing loop arg.
            Value::IntArray(_) | Value::RealArray(_) => {
                let len = crate::builtins::data::elem_count(&args[2]).map_err(sb)?;
                let vals = read_values(&args[2], 0, len).map_err(sb)?;
                let data = values_to_f64(&vals).map_err(sb)?;
                let loop_count = match args.get(3) {
                    Some(v) => v.to_int().map_err(sb)?,
                    None => 1,
                };
                if args.len() > 4 {
                    return Err(sb(crate::builtins::illegal()));
                }
                (data, loop_count)
            }
            // Form 2 — keyframes from DATA via "@label"; first DATA value is the count.
            Value::Str(label) => {
                let data = self.read_anim_data(label, stride)?;
                let loop_count = match args.get(3) {
                    Some(v) => v.to_int().map_err(sb)?,
                    None => 1,
                };
                if args.len() > 4 {
                    return Err(sb(crate::builtins::illegal()));
                }
                (data, loop_count)
            }
            // Form 3 — inline keyframes; a leftover trailing value (after whole keyframes) is
            // the loop count.
            _ => {
                let vals = values_to_f64(&args[2..]).map_err(sb)?;
                if vals.len() % stride == 1 {
                    let (kf, last) = vals.split_at(vals.len() - 1);
                    (kf.to_vec(), last[0] as i32)
                } else {
                    (vals, 1)
                }
            }
        };
        let scr = self.active_screen();
        self.bg[scr]
            .set_anim(layer, channel, relative, &data, loop_count)
            .map_err(|e| sb(spr::anim_err(e)))
    }

    /// `BGFUNC layer, @label` — bind a callback process name to a BG layer. Requires exactly
    /// 2 arguments (errnum 4); the layer ∉ 0..3 is errnum 10; a non-string label operand is
    /// errnum 8; an unresolvable label/process is errnum 4. The bound process runs later via
    /// `CALL BG` (with `CALLIDX` = the layer number) — dispatch is oracle-pending (M3-T6).
    fn do_bgfunc(&mut self, args: &[Value], ret_count: usize) -> Result<(), VmError> {
        if ret_count != 0 || args.len() != 2 {
            return Err(sb(crate::builtins::illegal()));
        }
        let layer = {
            let i = args[0].to_int().map_err(sb)?;
            if !sb_render::bg::BgState::in_range(i) {
                return Err(sb(crate::builtins::out_of_range()));
            }
            i as usize
        };
        let label = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(sb(crate::builtins::type_mismatch())),
        };
        let name = String::from_utf16_lossy(&label)
            .trim_start_matches('@')
            .to_ascii_uppercase();
        // The name must resolve to a code @label or a DEF-defined process; else errnum 4.
        let resolves = self.program.code_labels.iter().any(|(n, _)| *n == name)
            || self.program.functions.iter().any(|f| f.name.ident == name);
        if !resolves {
            return Err(sb(crate::builtins::illegal()));
        }
        let scr = self.active_screen();
        self.bg[scr].set_func(layer, Some(name));
        Ok(())
    }

    /// `SPANIM mgmt, target, data[, loop]` — define and start a sprite animation. The form
    /// is chosen by the third operand: a numeric **array** (form 1), an `"@label"` **string**
    /// pointing at DATA (form 2), or an inline numeric **argument list** (form 3). After the
    /// shared argcount>=3 / return-count==0 gate (errnum 4), the data is flattened to a
    /// `Time,Item[,Item],…` list and handed to [`spr::spanim`] (which checks mgmt/active/
    /// target and builds the keyframes). For the inline form the optional `loop` is the
    /// trailing value left over after a whole number of keyframes (the documented
    /// disambiguation by items-per-keyframe).
    fn do_spanim(&mut self, args: &[Value], ret_count: usize) -> Result<(), VmError> {
        use crate::builtins::data::read_values;
        use crate::builtins::sprite as spr;
        // Gate: a return value, or fewer than 3 arguments, is Illegal function call (4).
        if ret_count != 0 || args.len() < 3 {
            return Err(sb(crate::builtins::illegal()));
        }
        let mgmt_v = &args[0];
        let target_v = &args[1];
        // The channel (resolved from the target) gives items-per-keyframe for flattening.
        // SPANIM rejects a non-number/non-string target (e.g. an array) with errnum 4 — NOT
        // the errnum 8 that the shared `parse_target` (and BGANIM, hw_verified=8) use. The
        // array never reaches SPANIM's keyframe-data type check, which is the only errnum-8
        // path here (non-numeric keyframe value). hw_verified (2026-06-24, spanim.tsv):
        // `DIM AR[2]:SPANIM 0,AR,...` → errnum 4; `SPANIM 0,"XY",10,"S",5` → errnum 8.
        let (channel, _relative) = spr::parse_target(target_v).map_err(|e| {
            sb(if e.errnum == ERR_TYPE_MISMATCH {
                crate::builtins::illegal()
            } else {
                e
            })
        })?;
        let stride = 1 + spr::anim_items(channel);

        let (data, loop_count): (Vec<f64>, i32) = match &args[2] {
            // Form 1 — keyframes in a numeric array; an explicit trailing loop arg.
            Value::IntArray(_) | Value::RealArray(_) => {
                let len = crate::builtins::data::elem_count(&args[2]).map_err(sb)?;
                let vals = read_values(&args[2], 0, len).map_err(sb)?;
                let data = values_to_f64(&vals).map_err(sb)?;
                let loop_count = match args.get(3) {
                    Some(v) => v.to_int().map_err(sb)?,
                    None => 1,
                };
                if args.len() > 4 {
                    return Err(sb(crate::builtins::illegal()));
                }
                (data, loop_count)
            }
            // Form 2 — keyframes from DATA via "@label"; first DATA value is the count.
            // Real SB routes ANY string args[2] through its label-string parser, which treats
            // a leading `@` as a label lookup (errnum 14 if undefined) and a bare non-`@`
            // string as the first inline TIME (a symbol-string context → errnum 34 if
            // non-numeric). We mirror that by selecting form-2 only when the string starts
            // with `@`; otherwise the string falls through to the inline form-3 path, where
            // `parse_anim_time` raises errnum 34 for it. hw_verified 2026-06-26 (sb64u_form.tsv):
            // `SPANIM 0,"XY","@S"` → errnum 14 (undefined label); `SPANIM 0,"XY","S"` →
            // errnum 34 (bare string = inline time). The corpus confirms every form-2 label
            // string starts with `@` (e.g. `SPANIM 115,3,"@ANIM_M1"`).
            Value::Str(label) if label.first().is_some_and(|&c| c == b'@' as u16) => {
                let data = self.read_anim_data(label, stride)?;
                let loop_count = match args.get(3) {
                    Some(v) => v.to_int().map_err(sb)?,
                    None => 1,
                };
                if args.len() > 4 {
                    return Err(sb(crate::builtins::illegal()));
                }
                (data, loop_count)
            }
            // Form 3 — inline keyframes; a leftover trailing value (after whole keyframes) is
            // the loop count. The TIME slot of each keyframe is parsed as a symbol/label-string
            // context (real SB routes it through the same getter as GOTO/GOSUB), so a
            // non-numeric TIME raises errnum 34 (Illegal symbol string) — NOT the errnum 8 a
            // non-numeric ITEM raises (hw_verified 2026-06-26, sb64u.tsv). The trailing loop
            // count is a plain integer (errnum 8 on a non-numeric value).
            _ => {
                let rest = &args[2..];
                // A trailing lone value (rest.len() % stride == 1) is the LOOP count ONLY
                // when at least one full keyframe precedes it. With fewer than `stride`
                // values, position 0 is always a TIME (real SB parses the time slot before
                // the count check), so a bare `SPANIM 0,"XY","S"` raises errnum 34, not 8
                // (sb64u_edge.tsv: `one_bad_time` → 34; `one_time_only` → 4 count-check).
                let (n_full, leftover) = if rest.len() % stride == 1 && rest.len() > stride {
                    (rest.len() - 1, Some(rest.len() - 1))
                } else {
                    (rest.len(), None)
                };
                let mut data = Vec::with_capacity(n_full);
                let mut i = 0;
                while i < n_full {
                    let pos_in_kf = i % stride;
                    let v = &rest[i];
                    if pos_in_kf == 0 {
                        // TIME slot — symbol-string context: a non-numeric value here is
                        // errnum 34, not 8.
                        data.push(parse_anim_time(v).map_err(sb)?);
                    } else {
                        data.push(v.to_real().map_err(sb)?);
                    }
                    i += 1;
                }
                let loop_count = match leftover {
                    Some(j) => rest[j].to_int().map_err(sb)?,
                    None => 1,
                };
                (data, loop_count)
            }
        };
        let scr = self.active_screen();
        spr::spanim(&mut self.sprites[scr], mgmt_v, target_v, &data, loop_count).map_err(sb)
    }

    /// Read `SPANIM` form-2 keyframe data from the DATA sequence named by `@label`: the
    /// first DATA value is the keyframe count, then `stride` values (`Time,Item[,Item]`) per
    /// keyframe. Returns the flattened `Time,Item,…` list (without the count). An undefined
    /// label raises errnum 14; running off the DATA pool raises errnum 13.
    fn read_anim_data(&self, label: &[u16], stride: usize) -> Result<Vec<f64>, VmError> {
        let name = String::from_utf16_lossy(label)
            .trim_start_matches('@')
            .to_ascii_uppercase();
        let idx = self
            .program
            .data_labels
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, i)| *i)
            .ok_or(VmError::Sb {
                errnum: ERR_UNDEFINED_LABEL,
                line: 0,
            })?;
        let read_one = |k: usize| -> Result<f64, VmError> {
            let c = self.program.data.get(idx + k).ok_or(VmError::Sb {
                errnum: ERR_OUT_OF_DATA,
                line: 0,
            })?;
            const_to_value(c).to_real().map_err(sb)
        };
        let count = read_one(0)? as i32;
        let count = count.max(0) as usize;
        let mut out = Vec::with_capacity(count * stride);
        for k in 0..count * stride {
            out.push(read_one(1 + k)?);
        }
        Ok(out)
    }

    /// `SPFUNC mgmt, @Label` — bind a callback process name to a sprite. Requires exactly 2
    /// arguments (errnum 4); mgmt ∉ 0..511 is errnum 10; a non-string label operand is
    /// errnum 8; an unresolvable label/process is errnum 4. Binding does NOT require the
    /// slot to be `SPSET`. The bound process runs later via `CALL SPRITE` (oracle-pending).
    fn do_spfunc(&mut self, args: &[Value], ret_count: usize) -> Result<(), VmError> {
        if ret_count != 0 || args.len() != 2 {
            return Err(sb(crate::builtins::illegal()));
        }
        let slot = {
            let i = args[0].to_int().map_err(sb)?;
            if !SpriteState::in_range(i) {
                return Err(sb(crate::builtins::out_of_range()));
            }
            i as usize
        };
        let label = match &args[1] {
            Value::Str(s) => s.clone(),
            _ => return Err(sb(crate::builtins::type_mismatch())),
        };
        let name = String::from_utf16_lossy(&label)
            .trim_start_matches('@')
            .to_ascii_uppercase();
        // The name must resolve to a code @label or a DEF-defined process; else errnum 4.
        let resolves = self.program.code_labels.iter().any(|(n, _)| *n == name)
            || self.program.functions.iter().any(|f| f.name.ident == name);
        if !resolves {
            return Err(sb(crate::builtins::illegal()));
        }
        let scr = self.active_screen();
        self.sprites[scr].set_func(slot, Some(name));
        Ok(())
    }

    /// One `CALL SPRITE`/`CALL BG` dispatch step (M6-T6). Advances [`Vm::callidx`] and, if the
    /// next sprite/layer has a bound `SPFUNC`/`BGFUNC` process, invokes it and rewinds the PC to
    /// `here` (this `CallbackNext` op) so the sweep resumes after the callback returns. When the
    /// index runs past the table, the sweep ends and control falls through to the next statement.
    ///
    /// Modeled on osb `VM.d:CallSprite`/`CallBG` (the only structural reference — these are
    /// parser special forms with no body-readable disassembly): one shared `callidx` counter
    /// incremented once per step, the bound process called like a `GOSUB`/`DEF` so it returns to
    /// re-run the step. A `CALLIDX` read inside a process therefore yields its own
    /// management/layer number (documented). The final `callidx` is left at one-past the table
    /// (e.g. 4 after `CALL BG`), matching osb. Whether real SB clears `callidx`, raises on a
    /// bound-but-not-`SPSET` sprite, or shares the counter with a nested sweep is oracle-pending
    /// (queued, `bd:sb-interpreter-7td`).
    fn callback_next(&mut self, kind: CallbackKind, here: usize) -> Result<(), VmError> {
        self.callidx += 1;
        if !self.callback_active {
            return Ok(());
        }
        let max = match kind {
            CallbackKind::Sprite => sb_render::sprite::SPRITE_COUNT as i32,
            CallbackKind::Bg => sb_render::bg::BG_LAYER_COUNT as i32,
        };
        if self.callidx >= max {
            // Ran past the table: the sweep is done, fall through (PC already advanced).
            self.callback_active = false;
            return Ok(());
        }
        let idx = self.callidx as usize;
        let bound = match kind {
            CallbackKind::Sprite => self.sprites[self.active_screen()].func(idx),
            CallbackKind::Bg => self.bg[self.active_screen()].func(idx),
        };
        // Re-run this op after the dispatch (or after skipping an unbound index) so the sweep
        // keeps advancing — the osb `pc--` self-loop.
        self.pc = here;
        if let Some(name) = bound {
            self.dispatch_callback(&name)?;
        }
        Ok(())
    }

    /// Invoke a registered callback process by name (M6-T6 `CALL SPRITE`/`CALL BG`). The name
    /// was resolved + recorded by `SPFUNC`/`BGFUNC` (uppercase, no leading `@`), so it names a
    /// code `@label` (run like `GOSUB` — priority, matching the bind-time `@Label`-first lookup)
    /// or a `DEF` process (run like a 0-arg `CALL`). Either return path lands back at the
    /// `CallbackNext` op (`self.pc` is already `here`), continuing the sweep.
    fn dispatch_callback(&mut self, name: &str) -> Result<(), VmError> {
        if let Some(addr) = self
            .program
            .code_labels
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| *a)
        {
            self.push_gosub(self.pc)?;
            self.pc = addr;
            return Ok(());
        }
        let fname = Name::new(name.to_string(), Suffix::None);
        if let Some(idx) = self.program.function_index(&fname) {
            // `invoke_function` captures `self.pc` (== `here`) as the return address.
            return self.invoke_function(idx, 0, 0, false);
        }
        // A bound name that no longer resolves (e.g. its program was swapped out): skip it.
        Ok(())
    }

    /// `SPDEF` — manage the sprite definition-template table. The VM owns this (rather than
    /// the stateless dispatch) because the bulk forms read a numeric array (form 3) or a
    /// DATA `@label` sequence (form 4). Forms:
    /// - `ret_count > 0` (an `OUT` getter, form 5): read a template's fields into the OUT
    ///   variables (U,V then W,H then OX,OY then attr, in order — no intermediate skipping).
    /// - `ret_count == 0`, no args: reset the whole table (form 1).
    /// - first arg a numeric **array**: bulk-define from 7-element groups (form 3).
    /// - first arg a **string** `@label`: bulk-define from DATA (count, then 7 per template;
    ///   form 4).
    /// - first arg a numeric **scalar**: define (form 2) or copy-with-adjust (form 6).
    ///
    /// errnum 4 for a bad call shape, 10 for an out-of-range def/field, 31 for a bulk array
    /// whose element count is not a multiple of 7.
    fn do_spdef(&mut self, args: &[Value], ret_count: usize) -> Result<(), VmError> {
        use crate::builtins::data::{elem_count, read_values};
        use crate::builtins::sprite as spr;

        // Getter form (form 5): SPDEF defnum OUT U,V[,…].
        if ret_count > 0 {
            if args.len() != 1 || ret_count > 7 {
                return Err(sb(crate::builtins::illegal()));
            }
            let defnum = {
                let i = args[0].to_int().map_err(sb)?;
                if !(0..=sb_render::sprite::SPDEF_MAX).contains(&i) {
                    return Err(sb(crate::builtins::out_of_range()));
                }
                i as usize
            };
            let e = self.spdef.get(defnum);
            let fields = [e.u, e.v, e.w, e.h, e.origin_x, e.origin_y, e.attr];
            for &f in &fields[..ret_count] {
                self.stack.push(Value::Int(f));
            }
            return Ok(());
        }

        match args {
            // Form 1 — reset the whole (shared) table.
            [] => self.spdef.reset(),
            // Forms 3/4/2/6, dispatched on the first argument's type.
            [first, ..] => match first {
                // Form 3 — bulk define from a numeric array (7 elements per template).
                Value::IntArray(_) | Value::RealArray(_) => {
                    if args.len() != 1 {
                        return Err(sb(crate::builtins::illegal()));
                    }
                    let len = elem_count(first).map_err(sb)?;
                    if len % 7 != 0 {
                        return Err(VmError::Sb {
                            errnum: ERR_SUBSCRIPT,
                            line: 0,
                        });
                    }
                    let vals = read_values(first, 0, len).map_err(sb)?;
                    let data = values_to_f64(&vals).map_err(sb)?;
                    self.spdef_bulk(&data)?;
                }
                // Form 4 — bulk define from DATA named by "@label".
                Value::Str(label) => {
                    if args.len() != 1 {
                        return Err(sb(crate::builtins::illegal()));
                    }
                    let data = self.read_anim_data(label, 7)?;
                    self.spdef_bulk(&data)?;
                }
                // Forms 2/6 — single-template define or copy-with-adjust.
                _ => spr::spdef_scalar(&mut self.spdef, args).map_err(sb)?,
            },
        }
        Ok(())
    }

    /// Define templates 0,1,2,… from a flat list of 7-field groups (`SPDEF` bulk forms 3/4).
    /// The template count is clamped to the table size (4096); each group is range-validated
    /// (errnum 10).
    fn spdef_bulk(&mut self, data: &[f64]) -> Result<(), VmError> {
        use crate::builtins::sprite as spr;
        let count = (data.len() / 7).min(sb_render::sprite::SPDEF_TEMPLATE_COUNT);
        for i in 0..count {
            let entry = spr::spdef_entry_from_slice(&data[i * 7..i * 7 + 7]);
            spr::validate_spdef(&entry).map_err(sb)?;
            self.spdef.set(i, entry);
        }
        Ok(())
    }

    /// `ASSERT__ condition[, message$]` — the test-mode assertion (M1-T14). A truthy
    /// condition (non-zero number) is a no-op; a false (zero) condition halts the run with
    /// [`VmError::Assert`] carrying `message$` (empty if omitted). A bad argument count is
    /// an Illegal function call (4); a non-numeric condition is a Type mismatch (8). The
    /// `line` is filled by [`Vm::attach_line`] from the call site.
    fn call_assert(&mut self, args: &[Value]) -> Result<(), VmError> {
        let (cond, message) = match args {
            [c] => (c.deref(), String::new()),
            [c, m] => {
                let msg = match m.deref() {
                    Value::Str(s) => String::from_utf16_lossy(&s),
                    other => crate::builtins::format_number(&other).map_err(sb)?,
                };
                (c.deref(), msg)
            }
            _ => return Err(sb(crate::builtins::illegal())),
        };
        let truthy = match cond {
            Value::Int(i) => i != 0,
            Value::Real(r) => r != 0.0,
            _ => return Err(sb(crate::builtins::type_mismatch())),
        };
        if truthy {
            Ok(())
        } else {
            Err(VmError::Assert { message, line: 0 })
        }
    }

    /// `BACKCOLOR` — the SET form (statement, exactly 1 argument) stores the background
    /// color code; the GET form (function, 0 arguments) returns it. The hardware keeps only a
    /// 24-bit RGB border color: the GET round-trip drops the high (alpha) byte, so
    /// `BACKCOLOR RGB(64,128,128):?BACKCOLOR()` returns `&H408080` (= 4227200), not the
    /// `&HFF408080` the user passed — even a non-`0xFF` high byte (`BACKCOLOR &H7F112233` →
    /// `&H112233`) is stripped. We mask the stored value to 24 bits on SET so the kept color
    /// matches what the hardware retains. Any other call shape → errnum 4 (`backcolor.yaml`,
    /// hw_verified via sb-oracle batch 2026-06-23).
    fn backcolor(&mut self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if wants_value {
            if !args.is_empty() {
                return Err(crate::builtins::illegal());
            }
            Ok(Value::Int(self.back_color))
        } else {
            if args.len() != 1 {
                return Err(crate::builtins::illegal());
            }
            self.back_color = args[0].to_int()? & 0x00FF_FFFF;
            Ok(Value::Void)
        }
    }

    /// `FADE` — the screen fader (M4/M5 frame effect). The statement form `FADE color[,time]`
    /// sets (or animates to) the target ARGB fader color; the function form `FADE()` returns the
    /// current fader color including its alpha byte. Any other call shape → errnum 4; a negative
    /// time → errnum 10 (see `spec/instructions/FADE.yaml`).
    fn fade(&mut self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if wants_value {
            // Function form: FADE() with no arguments.
            if !args.is_empty() {
                return Err(crate::builtins::illegal());
            }
            return Ok(Value::Int(self.fader.current()));
        }
        match args.len() {
            1 => {
                let color = args[0].to_int()?;
                self.fader.set(color);
                Ok(Value::Void)
            }
            2 => {
                let color = args[0].to_int()?;
                let time = args[1].to_int()?;
                if time < 0 {
                    return Err(crate::builtins::out_of_range());
                }
                self.fader.fade_to(color, time as u64);
                Ok(Value::Void)
            }
            _ => Err(crate::builtins::illegal()),
        }
    }

    /// `FADECHK()` — returns 1 (TRUE) while a timed `FADE` animation is in progress and 0
    /// (FALSE) when idle/suspended/complete. Must be a no-argument function; any other shape
    /// → errnum 4.
    fn fadechk(&mut self, args: &[Value], wants_value: bool) -> Result<Value, RuntimeError> {
        if !wants_value || !args.is_empty() {
            return Err(crate::builtins::illegal());
        }
        Ok(Value::Int(if self.fader.is_animating() { 1 } else { 0 }))
    }

    /// `INPUT` (M1-T8): optionally print the prompt and `?`, read one line, split it on
    /// commas into `count` fields, parse each per its receiver type, and push the fields so
    /// the following `PopVar`s assign them in receiver order.
    fn do_input(
        &mut self,
        count: u8,
        question: bool,
        has_prompt: bool,
        types: &[VarType],
    ) -> Result<(), VmError> {
        if !self.input_prompt_printed {
            if has_prompt {
                let p = self.pop()?.deref();
                self.print_units(&p)?;
            }
            if question {
                self.active_console_mut().put_char(u16::from(b'?'));
            }
            self.input_prompt_printed = true;
        }

        if let Some(line) = self.input_buffer.next_line() {
            self.active_console_mut().newline(); // ENTER moves to the next line
            self.input_prompt_printed = false;
            let fields: Vec<&[u16]> = line.split(|&u| u == COMMA).collect();
            let mut values: Vec<Value> = Vec::with_capacity(count as usize);
            for i in 0..count as usize {
                let field = fields.get(i).copied().unwrap_or(&[]);
                let ty = types.get(i).copied().unwrap_or(VarType::Real);
                values.push(parse_input_field(field, ty));
            }
            // The receivers' `PopVar`s pop top-first in declaration order, so push reversed
            // (the first receiver's value ends on top).
            for v in values.into_iter().rev() {
                self.stack.push(v);
            }
            return Ok(());
        }

        if self.interactive {
            // No line yet: yield and retry this opcode next frame.
            self.pc -= 1;
            self.input_wait = true;
            self.yielded = true;
            return Ok(());
        }

        // Headless fallback: empty line.
        self.active_console_mut().newline();
        self.input_prompt_printed = false;
        let mut values: Vec<Value> = Vec::with_capacity(count as usize);
        for i in 0..count as usize {
            let ty = types.get(i).copied().unwrap_or(VarType::Real);
            values.push(parse_input_field(&[], ty));
        }
        for v in values.into_iter().rev() {
            self.stack.push(v);
        }
        Ok(())
    }

    /// `LINPUT` (M1-T8): optionally print the prompt, then read one whole line (commas
    /// kept) into the single following string receiver.
    fn do_linput(&mut self, has_prompt: bool) -> Result<(), VmError> {
        if !self.input_prompt_printed {
            if has_prompt {
                let p = self.pop()?.deref();
                self.print_units(&p)?;
            }
            self.input_prompt_printed = true;
        }

        if let Some(line) = self.input_buffer.next_line() {
            self.active_console_mut().newline();
            self.input_prompt_printed = false;
            self.stack.push(Value::Str(line));
            return Ok(());
        }

        if self.interactive {
            self.pc -= 1;
            self.input_wait = true;
            self.yielded = true;
            return Ok(());
        }

        self.active_console_mut().newline();
        self.input_prompt_printed = false;
        self.stack.push(Value::Str(SbStr::new()));
        Ok(())
    }

    /// Print a value's formatted code units to the console (shared by INPUT/LINPUT prompts).
    fn print_units(&mut self, v: &Value) -> Result<(), VmError> {
        let units = crate::builtins::console::format_print_item(v).map_err(sb)?;
        for u in units {
            self.active_console_mut().put_char(u);
        }
        Ok(())
    }

    fn return_func(&mut self, has_value: bool) -> Result<(), VmError> {
        // The return value (if any) is on top of the operand stack.
        let ret = if has_value { Some(self.pop()?) } else { None };
        let frame = self.frames.pop().ok_or(VmError::Sb {
            errnum: ERR_RETURN_WITHOUT_GOSUB,
            line: 0,
        })?;
        let f = &self.program.functions[frame.func];
        // Read OUT-param results from their frame locals, in declaration order. This must
        // happen while the function's own slot is still active (its `functions` table).
        let nparams = f.params.len();
        let out_vals: Vec<Value> = (0..f.out_params.len())
            .map(|i| frame.locals[nparams + i].borrow().clone())
            .collect();
        // Cross-slot `COMMON DEF` return (M6-T6): swap the caller's slot context back
        // before resuming at its `return_pc` (an index into the caller's program).
        if let Some(caller) = frame.caller_slot {
            self.activate_slot(caller);
        }
        self.pc = frame.return_pc;
        // Leave OUT results for the caller's pop targets (last OUT arg ends on top).
        for v in out_vals {
            self.stack.push(v);
        }
        // Leave the return value only if the caller wanted it.
        if let Some(v) = ret {
            if frame.wants_value {
                self.stack.push(v);
            }
        }
        Ok(())
    }

    // -- helpers ---------------------------------------------------------------

    fn push_gosub(&mut self, addr: usize) -> Result<(), VmError> {
        if self.gosub.len() >= GOSUB_STACK_LIMIT {
            return Err(VmError::Sb {
                errnum: ERR_STACK_OVERFLOW,
                line: 0,
            });
        }
        self.gosub.push(addr);
        Ok(())
    }

    /// The storage cell a [`VarRef`] addresses (a global, or a current-frame local).
    fn cell(&self, vref: VarRef) -> Result<&Cell, VmError> {
        match vref {
            VarRef::Global(i) => self.globals.get(i as usize).ok_or(VmError::Sb {
                errnum: ERR_UNDEFINED_FUNCTION,
                line: 0,
            }),
            VarRef::Local(i) => self
                .frames
                .last()
                .and_then(|fr| fr.locals.get(i as usize))
                .ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                }),
        }
    }

    /// The declared suffix of a variable (drives `PopVar` coercion).
    fn var_suffix(&self, vref: VarRef) -> Result<Suffix, VmError> {
        match vref {
            VarRef::Global(i) => self
                .program
                .globals
                .get(i as usize)
                .map(|v| v.name.suffix)
                .ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                }),
            VarRef::Local(i) => {
                let func = self.frames.last().map(|f| f.func).ok_or(VmError::Sb {
                    errnum: ERR_UNDEFINED_FUNCTION,
                    line: 0,
                })?;
                self.program.functions[func]
                    .locals
                    .get(i as usize)
                    .map(|v| v.name.suffix)
                    .ok_or(VmError::Sb {
                        errnum: ERR_UNDEFINED_FUNCTION,
                        line: 0,
                    })
            }
        }
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::Sb {
            errnum: ERR_STACK_UNDERFLOW,
            line: 0,
        })
    }

    /// Pop a condition and test its truthiness (nonzero).
    fn truthy(&mut self) -> Result<bool, VmError> {
        Ok(self.pop()?.to_real().map_err(sb)? != 0.0)
    }

    /// Peek the top condition's truthiness without consuming it.
    fn peek_truthy(&self) -> Result<bool, VmError> {
        let v = self.stack.last().ok_or(VmError::Sb {
            errnum: ERR_STACK_UNDERFLOW,
            line: 0,
        })?;
        Ok(v.to_real().map_err(sb)? != 0.0)
    }

    /// Pop a string and read it as a `@label` name (folded, leading `@` stripped).
    fn pop_label_name(&mut self) -> Result<String, VmError> {
        let v = self.pop()?;
        let s = v.as_str().map_err(sb)?;
        let text = String::from_utf16_lossy(s);
        Ok(text.trim_start_matches('@').to_ascii_uppercase())
    }

    /// Resolve a popped `@label` string to a code address.
    fn resolve_code_label(&mut self) -> Result<usize, VmError> {
        let label = self.pop_label_name()?;
        self.program
            .code_labels
            .iter()
            .find(|(n, _)| *n == label)
            .map(|(_, a)| *a)
            .ok_or(VmError::Sb {
                errnum: ERR_UNDEFINED_LABEL,
                line: 0,
            })
    }

    /// Stamp the source line of the op at `pc` onto a runtime error (for `ERRLINE`).
    fn attach_line(&self, e: VmError, pc: usize) -> VmError {
        match e {
            VmError::Sb { errnum, line: 0 } => {
                let line = self.program.locs.get(pc).map(|l| l.line).unwrap_or(0);
                VmError::Sb { errnum, line }
            }
            VmError::Assert { message, line: 0 } => {
                let line = self.program.locs.get(pc).map(|l| l.line).unwrap_or(0);
                VmError::Assert { message, line }
            }
            other => other,
        }
    }
}

/// The UTF-16 code unit for `,`, the `INPUT` field delimiter.
const COMMA: u16 = b',' as u16;

/// Parse one `INPUT` field into the value its receiver expects: a string receiver keeps the
/// raw text; a numeric receiver parses a number (integer if it has no fractional/exponent
/// part, else a Double), defaulting to `0` when the field is not a valid number (a degraded
/// stand-in for SmileBASIC's interactive "?Redo from start" re-prompt — queued).
fn parse_input_field(field: &[u16], ty: VarType) -> Value {
    match ty {
        VarType::Str => Value::Str(field.to_vec()),
        VarType::Int => Value::Int(parse_input_number(field).to_int().unwrap_or(0)),
        VarType::Real => parse_input_number(field),
    }
}

/// Parse a numeric `INPUT` field, returning an Integer when it parses as `i32` and a Double
/// otherwise (or Integer `0` when it is not a number).
fn parse_input_number(field: &[u16]) -> Value {
    let s = String::from_utf16_lossy(field);
    let s = s.trim();
    if let Ok(i) = s.parse::<i32>() {
        Value::Int(i)
    } else if let Ok(r) = s.parse::<f64>() {
        Value::Real(r)
    } else {
        Value::Int(0)
    }
}

/// Parse a `USE`/`EXEC` resource-string operand `"[PRGn:]filename"` into its
/// `(slot, filename)` parts. The resource type (when a `TYPE:` prefix is present) must be a
/// program slot — `PRG`/`PRG0`/`PRG1`/`PRG2`/`PRG3`. Any other prefix — an unknown family
/// (`FOO`) or a PRG index past the family (`PRG4`/`PRG5`) — or an empty filename is an Illegal
/// function call (errnum 4). (hw_verified, sb-oracle 2026-06-23: `USE "PRG4:X"` / `USE "FOO:X"`
/// / `USE "PRG1:"` all raise 4 — note this differs from the `SAVE` resolver, where an
/// out-of-family index raises Out of range 10.) A bare filename (no `:`) yields `slot = None`
/// (the default slot, selected by the loader).
fn parse_prg_operand(s: &str) -> Result<(Option<u8>, &str), u32> {
    match s.split_once(':') {
        Some((ty, name)) => {
            let slot = match ty.to_ascii_uppercase().as_str() {
                "PRG" | "PRG0" => 0u8,
                "PRG1" => 1,
                "PRG2" => 2,
                "PRG3" => 3,
                _ => return Err(ERR_ILLEGAL_FUNCTION_CALL),
            };
            if name.is_empty() {
                return Err(ERR_ILLEGAL_FUNCTION_CALL);
            }
            Ok((Some(slot), name))
        }
        None if s.is_empty() => Err(ERR_ILLEGAL_FUNCTION_CALL),
        None => Ok((None, s)),
    }
}

/// Materialise a bytecode [`Const`] into a runtime [`Value`].
fn const_to_value(c: &Const) -> Value {
    match c {
        Const::Int(i) => Value::Int(*i),
        Const::Real(r) => Value::Real(*r),
        Const::Str(s) => Value::Str(s.clone()),
    }
}

/// Convert a slice of numeric [`Value`]s to `f64` (SPANIM keyframe data); a non-numeric
/// value is a Type mismatch (8).
fn values_to_f64(vals: &[Value]) -> Result<Vec<f64>, RuntimeError> {
    vals.iter().map(|v| v.to_real()).collect()
}

/// Parse an inline `SPANIM`/`BGANIM` keyframe **TIME** slot to `f64`. Real SmileBASIC
/// routes this slot through the same symbol/label-string getter as `GOTO`/`GOSUB` rather
/// than the numeric value getter the keyframe ITEMs use, so a non-numeric value here is
/// `Illegal symbol string` (errnum 34) — NOT the `Type mismatch` (errnum 8) a non-numeric
/// ITEM raises. hw_verified 2026-06-26 (sb64u.tsv): `SPANIM 0,"XY","S",5,5` → errnum 34
/// errline 1 (string target); `SPANIM 0,1,"S",5` → errnum 34 errline 1 (numeric target).
/// Only the inline form-3 path raises 34 — the form-1 array path raises 8 for the same
/// bad time (sb64u_arr.tsv), so callers must route inline times through this helper and
/// array times through [`values_to_f64`].
fn parse_anim_time(v: &Value) -> Result<f64, RuntimeError> {
    match v {
        Value::Int(i) => Ok(*i as f64),
        Value::Real(d) => Ok(*d),
        _ => Err(crate::builtins::illegal_symbol_string()),
    }
}

/// Wrap a [`RuntimeError`] (no line yet) as a [`VmError::Sb`]; the run loop fills the
/// line via [`Vm::attach_line`].
fn sb(e: RuntimeError) -> VmError {
    VmError::Sb {
        errnum: e.errnum,
        line: 0,
    }
}

/// Coerce a seed-ID argument to a series index, validating the `0..=7` range. A string →
/// Type mismatch (8); out of range → Out of range (10) (disasm `seed_id >= 8 → errnum 10`).
fn rng_seed_id(v: &Value) -> Result<usize, RuntimeError> {
    let id = v.to_int()?;
    if (0..=7).contains(&id) {
        Ok(id as usize)
    } else {
        Err(crate::builtins::out_of_range())
    }
}

/// Extract a [`Value::Ref`], erroring (errnum 8) if the operand is not a reference.
fn as_ref(v: Value) -> Result<Value, VmError> {
    match v {
        Value::Ref(_) | Value::ElemRef(_) => Ok(v),
        _ => Err(VmError::Sb {
            errnum: ERR_TYPE_MISMATCH,
            line: 0,
        }),
    }
}

// -- operator evaluation -------------------------------------------------------

/// Evaluate a binary operator on two runtime values (see the module-level operator
/// semantics). Errors carry only an errnum; the VM attaches the line.
fn operate(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use BinOp::*;
    match op {
        Add => add(lhs, rhs),
        Sub => num_arith(lhs, rhs, i32::wrapping_sub, |a, b| a - b),
        Mul => mul(lhs, rhs),
        Div => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            if y == 0.0 {
                Err(RuntimeError::new(ERR_DIVIDE_BY_ZERO))
            } else {
                Ok(Value::Real(x / y))
            }
        }
        IntDiv => int_div_mod(lhs, rhs, true),
        Mod => int_div_mod(lhs, rhs, false),
        And => bitwise(lhs, rhs, |a, b| a & b),
        Or => bitwise(lhs, rhs, |a, b| a | b),
        Xor => bitwise(lhs, rhs, |a, b| a ^ b),
        Shl => shift(lhs, rhs, true),
        Shr => shift(lhs, rhs, false),
        Eq | Ne | Lt | Le | Gt | Ge => compare(op, lhs, rhs),
        LAnd => Ok(Value::Int((truthy(&lhs)? && truthy(&rhs)?) as i32)),
        LOr => Ok(Value::Int((truthy(&lhs)? || truthy(&rhs)?) as i32)),
    }
}

/// `+`: numeric add (Integer wraps; a Double promotes) or string concatenation.
fn add(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (&lhs, &rhs) {
        (Value::Str(a), Value::Str(b)) => {
            let mut s = a.clone();
            s.extend_from_slice(b);
            Ok(Value::Str(s))
        }
        _ if lhs.is_numeric() && rhs.is_numeric() => {
            num_arith(lhs, rhs, i32::wrapping_add, |a, b| a + b)
        }
        _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    }
}

/// `*`: numeric multiply (Integer wraps; a Double promotes) or string repetition
/// (`"A"*3` = "AAA"; `3*"A"` = "AAA"). A non-integer repeat count is type mismatch.
fn mul(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (&lhs, &rhs) {
        (Value::Str(s), Value::Int(n)) | (Value::Int(n), Value::Str(s)) => {
            let count = (*n).max(0) as usize;
            let mut out = Vec::with_capacity(s.len().saturating_mul(count));
            for _ in 0..count {
                out.extend_from_slice(s);
            }
            Ok(Value::Str(out))
        }
        _ if lhs.is_numeric() && rhs.is_numeric() => {
            num_arith(lhs, rhs, i32::wrapping_mul, |a, b| a * b)
        }
        _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    }
}

/// Integer-wrapping `+`/`-`/`*`; promote to Double if either operand is a Double.
fn num_arith(
    lhs: Value,
    rhs: Value,
    fi: fn(i32, i32) -> i32,
    fr: fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (&lhs, &rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(fi(*a, *b))),
        _ => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            Ok(Value::Real(fr(x, y)))
        }
    }
}

/// `DIV`/`MOD`: truncate both operands toward zero to `i32`, then integer
/// quotient/remainder. A (truncated) zero divisor → errnum 7.
fn int_div_mod(lhs: Value, rhs: Value, is_div: bool) -> Result<Value, RuntimeError> {
    let (x, y) = (lhs.to_int()?, rhs.to_int()?);
    if y == 0 {
        return Err(RuntimeError::new(ERR_DIVIDE_BY_ZERO));
    }
    Ok(Value::Int(if is_div {
        x.wrapping_div(y)
    } else {
        x.wrapping_rem(y)
    }))
}

/// `AND`/`OR`/`XOR`: truncate both operands toward zero to `i32`, then the bitwise op.
/// The result is always Integer.
fn bitwise(lhs: Value, rhs: Value, f: fn(i32, i32) -> i32) -> Result<Value, RuntimeError> {
    let (x, y) = (lhs.to_int()?, rhs.to_int()?);
    Ok(Value::Int(f(x, y)))
}

/// `<<` / `>>`: truncate both operands toward zero to `i32`, then shift by the count.
/// hw_verified (sb-oracle 2026-06-24): the shift count is NOT masked mod 32.
/// - **Left** (`<<`): a count `< 0` or `>= 32` yields `0` (`1<<32=0`, `1<<33=0`,
///   `2<<-1=0`); otherwise the low bits are kept (`1<<31=-2147483648`, `-1<<1=-2`,
///   `3<<1=6`).
/// - **Right** (`>>`): ARITHMETIC (sign-extending); a count `< 0` yields `0`
///   (`256>>-1=0`), a count `>= 32` saturates to the sign bit (clamped to 31:
///   `-1>>32=-1`, `5>>32=0`); otherwise `-1>>1=-1`, `256>>2=64`,
///   `-2147483648>>1=-1073741824`.
fn shift(lhs: Value, rhs: Value, left: bool) -> Result<Value, RuntimeError> {
    let (x, n) = (lhs.to_int()?, rhs.to_int()?);
    let r = if left {
        if (0..32).contains(&n) {
            x.wrapping_shl(n as u32)
        } else {
            0
        }
    } else if n < 0 {
        0
    } else {
        x >> n.min(31)
    };
    Ok(Value::Int(r))
}

/// `+`/`-`/`*` in a context where at least one operand is statically Real/Number-typed
/// (a suffix-less or `#` variable/array element, a Real literal, a `/` result, …). The
/// compiler selects this path via `is_real_typed` (see `compiler.rs`).
///
/// hw_verified (sb-oracle 2026-06-24): when both operands are Integer at runtime and the
/// exact result overflows `i32`, it PROMOTES to Double rather than wrapping
/// (`A=2147483647:A+1 → 2147483648.0`, `A=2000000000:A*2 → 4e9`) — whereas the same op
/// on two statically-Integer (`%`/integer-literal/bit-op) operands wraps mod 2³²
/// (`A%=2000000000:A%*2 → -294967296`, handled by [`num_arith`] via [`operate`]). Any
/// non-(Int,Int) case (a Real operand, string concat, a type error) is identical to the
/// plain operator, so it delegates to [`operate`].
fn operate_promote(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let (Value::Int(a), Value::Int(b)) = (&lhs, &rhs) {
        let (a, b) = (*a as i64, *b as i64);
        // i32 OP i32 in i64 cannot itself overflow (|product| ≤ 2⁶²), so this is exact.
        let r = match op {
            BinOp::Add => Some(a + b),
            BinOp::Sub => Some(a - b),
            BinOp::Mul => Some(a * b),
            _ => None,
        };
        if let Some(r) = r {
            return Ok(narrow_or_real(r));
        }
    }
    operate(op, lhs, rhs)
}

/// Narrow an exact `i64` arithmetic result back to a [`Value`]: Integer if it fits
/// `i32`, otherwise Double (the overflow-promotion rule).
fn narrow_or_real(r: i64) -> Value {
    if (i32::MIN as i64..=i32::MAX as i64).contains(&r) {
        Value::Int(r as i32)
    } else {
        Value::Real(r as f64)
    }
}

/// Comparisons yield Integer `1`/`0`. Strings compare by UTF-16 code units; numbers
/// by `f64` value; a string-vs-number comparison is Type mismatch (errnum 8).
fn compare(op: BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let ord = match (&lhs, &rhs) {
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        _ if lhs.is_numeric() && rhs.is_numeric() => {
            let (x, y) = (lhs.to_real()?, rhs.to_real()?);
            x.partial_cmp(&y).unwrap_or(Ordering::Greater)
        }
        _ => return Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
    };
    let res = match op {
        BinOp::Eq => ord == Ordering::Equal,
        BinOp::Ne => ord != Ordering::Equal,
        BinOp::Lt => ord == Ordering::Less,
        BinOp::Le => ord != Ordering::Greater,
        BinOp::Gt => ord == Ordering::Greater,
        BinOp::Ge => ord != Ordering::Less,
        _ => unreachable!("compare only handles comparison ops"),
    };
    Ok(Value::Int(res as i32))
}

/// Evaluate a unary operator.
fn unary(op: UnOp, v: Value) -> Result<Value, RuntimeError> {
    match op {
        UnOp::Neg => match v {
            Value::Int(i) => Ok(Value::Int(i.wrapping_neg())),
            Value::Real(r) => Ok(Value::Real(-r)),
            _ => Err(RuntimeError::new(ERR_TYPE_MISMATCH)),
        },
        // NOT: bitwise complement of the integer-truncated operand.
        UnOp::Not => Ok(Value::Int(!v.to_int()?)),
        // `!`: logical not — 1 if the operand is zero, else 0.
        UnOp::LNot => Ok(Value::Int((v.to_real()? == 0.0) as i32)),
    }
}

/// Truthiness of a value: a nonzero number is true (string → Type mismatch).
fn truthy(v: &Value) -> Result<bool, RuntimeError> {
    Ok(v.to_real()? != 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;

    /// Compile + run a program to completion, returning the VM for inspection.
    fn run(src: &str) -> Vm {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect("run");
        vm
    }

    fn run_err(src: &str) -> VmError {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect_err("expected a runtime error")
    }

    /// Compile + run with the M1-T7 builtin registry (so bare names like `PI` resolve
    /// as calls), returning the VM for inspection.
    fn run_b(src: &str) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect("run");
        vm
    }

    /// Compile (with the builtin registry) + run, returning the error it halts with.
    fn run_b_err(src: &str) -> VmError {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.run().expect_err("expected a runtime error")
    }

    // ---- ASSERT__ (test-mode builtin, M1-T14) ----

    #[test]
    fn assert_true_is_a_noop() {
        // A truthy condition lets the program continue to the PRINT.
        let vm = run_b(r#"ASSERT__ 1,"msg":PRINT "OK""#);
        assert_eq!(vm.console_text(), "OK");
    }

    #[test]
    fn assert_false_halts_with_message_and_line() {
        let err = run_b_err("PRINT 1\nASSERT__ 2==3,\"twothree\"\nPRINT 9");
        match err {
            VmError::Assert { message, line } => {
                assert_eq!(message, "twothree");
                assert_eq!(line, 2); // the ASSERT__ is on source line 2
            }
            other => panic!("expected VmError::Assert, got {other:?}"),
        }
        // A failed assertion is NOT a SmileBASIC error, so it carries no ERRNUM.
        assert_eq!(run_b_err(r#"ASSERT__ 0,"x""#).errnum(), None);
    }

    #[test]
    fn assert_message_is_optional() {
        // One-arg form: false condition still halts, with an empty message.
        match run_b_err("ASSERT__ 0") {
            VmError::Assert { message, .. } => assert_eq!(message, ""),
            other => panic!("expected VmError::Assert, got {other:?}"),
        }
    }

    #[test]
    fn assert_bad_arg_count_is_illegal_function_call() {
        // Zero / three args → Illegal function call (errnum 4).
        assert_eq!(run_b_err("ASSERT__").errnum(), Some(4));
        assert_eq!(run_b_err(r#"ASSERT__ 1,"a",2"#).errnum(), Some(4));
    }

    #[test]
    fn assert_string_condition_is_type_mismatch() {
        assert_eq!(run_b_err(r#"ASSERT__ "x","msg""#).errnum(), Some(8));
    }

    // ---- SORT / RSORT (array data-ops, M1-T14) ----

    #[test]
    fn sort_reorders_key_and_parallel_arrays() {
        // A is the key; B is reordered by A's permutation (parallel-array sort).
        let vm = run_b(
            "DIM A[3]:DIM B[3]\nA[0]=3:A[1]=1:A[2]=2\nB[0]=30:B[1]=10:B[2]=20\nSORT A,B\n\
             PRINT A[0];A[1];A[2];\",\";B[0];B[1];B[2]",
        );
        assert_eq!(vm.console_text(), "123,102030");
    }

    #[test]
    fn rsort_is_descending() {
        let vm = run_b("DIM A[3]\nA[0]=1:A[1]=3:A[2]=2\nRSORT A\nPRINT A[0];A[1];A[2]");
        assert_eq!(vm.console_text(), "321");
    }

    #[test]
    fn sort_is_stable_rsort_is_its_reverse() {
        // SORT is a STABLE ascending sort: the two equal keys (the 1s) keep their order,
        // so the parallel array's tied entries stay 1,2 → B = 1,2,3,4. RSORT is the EXACT
        // REVERSE of SORT (not a stable descending sort): the tied entries swap → B =
        // 4,3,2,1. hw_verified (sb-oracle 2026-06-23); otya_test.sb3 STABLE[R]SORTTEST.
        let setup = "DIM A[4]:DIM B[4]\nA[0]=2:A[1]=3:A[2]=1:A[3]=1\nB[0]=3:B[1]=4:B[2]=1:B[3]=2\n";
        let asc = run_b(&format!("{setup}SORT A,B\nPRINT B[0];B[1];B[2];B[3]"));
        assert_eq!(asc.console_text(), "1234");
        let desc = run_b(&format!("{setup}RSORT A,B\nPRINT B[0];B[1];B[2];B[3]"));
        assert_eq!(desc.console_text(), "4321");
    }

    #[test]
    fn sort_leading_start_count_restricts_the_range() {
        // SORT 1,2 sorts only A[1..3); A[0] and A[3] stay in place.
        let vm =
            run_b("DIM A[4]\nA[0]=4:A[1]=3:A[2]=2:A[3]=1\nSORT 1,2,A\nPRINT A[0];A[1];A[2];A[3]");
        assert_eq!(vm.console_text(), "4231");
    }

    #[test]
    fn sort_string_key_is_lexical() {
        let vm = run_b(
            "DIM S$[3]\nS$[0]=\"c\":S$[1]=\"a\":S$[2]=\"b\"\nSORT S$\nPRINT S$[0];S$[1];S$[2]",
        );
        assert_eq!(vm.console_text(), "abc");
    }

    #[test]
    fn sort_count_past_end_is_out_of_range() {
        // start+count beyond the key array's length → Out of range (10).
        assert_eq!(run_b_err("DIM A[3]\nSORT 0,5,A").errnum(), Some(10));
    }

    #[test]
    fn sort_non_array_key_is_type_mismatch() {
        // A non-numeric scalar where an array is wanted → Type mismatch (8). (A numeric
        // scalar is instead consumed as a leading start/count number — see below.)
        assert_eq!(run_b_err(r#"SORT "x""#).errnum(), Some(8));
    }

    #[test]
    fn sort_without_a_key_array_is_illegal_function_call() {
        // A lone numeric and no array operand → Illegal function call (4).
        assert_eq!(run_b_err("VAR A=3\nSORT A").errnum(), Some(4));
    }

    // ---- COPY / FILL (block ops, M1-T14) ----
    // hw_verified expects from spec/instructions/{copy,fill}.yaml (sb-oracle 2026-06-22).

    #[test]
    fn copy_array_to_array() {
        let vm = run_b("DIM S[3]\nS[0]=1:S[1]=2:S[2]=3\nDIM D[3]\nCOPY D,S\nPRINT D[0];D[1];D[2]");
        assert_eq!(vm.console_text(), "123");
    }

    #[test]
    fn copy_auto_extends_a_1d_destination() {
        // A too-small 1D destination grows to fit the source (LEN(D) → 3).
        let vm = run_b("DIM S[3]\nDIM D[0]\nCOPY D,S\nPRINT LEN(D)");
        assert_eq!(vm.console_text(), "3");
    }

    #[test]
    fn copy_dest_offset_and_count() {
        // COPY D,1,S writes from D[1]; COPY D,S,0,2 copies only the first 2 (D[2] stays 0).
        let off = run_b("DIM S[2]\nS[0]=7:S[1]=8\nDIM D[4]\nCOPY D,1,S\nPRINT D[1];D[2]");
        assert_eq!(off.console_text(), "78");
        let cnt =
            run_b("DIM S[3]\nS[0]=1:S[1]=2:S[2]=3\nDIM D[3]\nCOPY D,S,0,2\nPRINT D[0];D[1];D[2]");
        assert_eq!(cnt.console_text(), "120");
    }

    #[test]
    fn copy_five_arg_form_uses_src_offset() {
        // COPY D,1,S,2,2 → D[1..3) = S[2..4) = 3,4.
        let vm = run_b(
            "DIM S[4]\nS[0]=1:S[1]=2:S[2]=3:S[3]=4\nDIM D[4]\nCOPY D,1,S,2,2\nPRINT D[1];D[2]",
        );
        assert_eq!(vm.console_text(), "34");
    }

    #[test]
    fn copy_from_data_label() {
        // Form 2: read a DATA sequence named by "@Label" into the destination.
        let vm = run_b("DIM D[5]\nCOPY D,\"@SRC\"\nPRINT D[0];D[4]\n@SRC\nDATA 5,1,1,2,4");
        assert_eq!(vm.console_text(), "54");
    }

    #[test]
    fn copy_numeric_into_string_array_is_type_mismatch() {
        assert_eq!(
            run_b_err("DIM A[1]\nDIM S$[1]\nCOPY A,S$").errnum(),
            Some(8)
        );
    }

    #[test]
    fn copy_from_undefined_label_is_undefined_label() {
        assert_eq!(run_b_err("DIM D[2]\nCOPY D,\"@NOPE\"").errnum(), Some(14));
    }

    #[test]
    fn copy_from_data_short_is_out_of_data() {
        // Default count = dest element count (3) but only 2 DATA items exist → Out of DATA (13).
        assert_eq!(
            run_b_err("DIM D[3]\nCOPY D,\"@SRC\"\n@SRC\nDATA 1,2").errnum(),
            Some(13)
        );
    }

    #[test]
    fn fill_all_and_subrange() {
        let all = run_b("DIM A[3]\nFILL A,9\nPRINT A[0];A[1];A[2]");
        assert_eq!(all.console_text(), "999");
        let sub = run_b("DIM A[4]\nFILL A,7,1,2\nPRINT A[0];A[1];A[2];A[3]");
        assert_eq!(sub.console_text(), "0770");
    }

    #[test]
    fn fill_string_array() {
        let vm = run_b("DIM S$[2]\nFILL S$,\"x\"\nPRINT S$[0];S$[1]");
        assert_eq!(vm.console_text(), "xx");
    }

    #[test]
    fn fill_value_type_mismatch_is_8() {
        assert_eq!(run_b_err("DIM A[2]\nFILL A,\"x\"").errnum(), Some(8));
    }

    #[test]
    fn fill_past_the_end_is_out_of_range() {
        // offset+count beyond the array bounds → Out of range (10).
        // hw_verified sb-oracle 2026-06-26 (s_t4c): oob offset/count/neg all → 10.
        assert_eq!(run_b_err("DIM A[3]\nFILL A,1,2,5").errnum(), Some(10));
    }

    // ---- PUSH / POP / SHIFT / UNSHIFT (stack/queue ops, M1-T14) ----
    // hw_verified expects from spec/instructions/{push,pop,shift,unshift}.yaml.

    #[test]
    fn push_grows_and_pop_is_lifo() {
        let vm = run_b("DIM A[0]\nPUSH A,1\nPUSH A,2\nX=POP(A)\nY=POP(A)\nPRINT X;Y;LEN(A)");
        assert_eq!(vm.console_text(), "210");
    }

    #[test]
    fn push_appends_at_end() {
        let vm = run_b("DIM A[2]\nPUSH A,7\nPRINT A[2];LEN(A)");
        assert_eq!(vm.console_text(), "73");
    }

    #[test]
    fn shift_is_fifo_and_compacts() {
        let vm = run_b("DIM A[0]\nPUSH A,1\nPUSH A,2\nX=SHIFT(A)\nY=SHIFT(A)\nPRINT X;Y");
        assert_eq!(vm.console_text(), "12");
    }

    #[test]
    fn unshift_inserts_at_front() {
        let vm = run_b("DIM A[0]\nPUSH A,1\nUNSHIFT A,2\nPRINT A[0];A[1]");
        assert_eq!(vm.console_text(), "21");
    }

    #[test]
    fn push_pop_on_string_variable_is_char_array() {
        // The character-array form: PUSH appends to the string, POP/SHIFT remove a char.
        let vm = run_b("S$=\"AB\"\nPUSH S$,\"CD\"\nPRINT S$");
        assert_eq!(vm.console_text(), "ABCD");
        let vm = run_b("S$=\"ABC\"\nX$=POP(S$)\nPRINT X$;\",\";S$");
        assert_eq!(vm.console_text(), "C,AB");
        let vm = run_b("S$=\"ABC\"\nX$=SHIFT(S$)\nPRINT X$;\",\";S$");
        assert_eq!(vm.console_text(), "A,BC");
        let vm = run_b("S$=\"C\"\nUNSHIFT S$,\"TXT:\"\nPRINT S$");
        assert_eq!(vm.console_text(), "TXT:C");
    }

    #[test]
    fn pop_real_array_keeps_double() {
        // POP returns the array element type — a Double array yields a Double.
        let vm = run_b("DIM A#[0]\nPUSH A#,1.5\nPRINT POP(A#)");
        assert_eq!(vm.console_text(), "1.5");
    }

    #[test]
    fn push_string_onto_numeric_array_is_type_mismatch() {
        assert_eq!(run_b_err("DIM A[1]\nPUSH A,\"x\"").errnum(), Some(8));
    }

    #[test]
    fn push_onto_numeric_scalar_is_type_mismatch() {
        assert_eq!(run_b_err("X=5\nPUSH X,1").errnum(), Some(8));
    }

    #[test]
    fn pop_on_numeric_scalar_is_type_mismatch() {
        assert_eq!(run_b_err("X=5\nY=POP(X)").errnum(), Some(8));
    }

    #[test]
    fn pop_empty_array_is_subscript_out_of_range() {
        assert_eq!(run_b_err("DIM A[0]\nX=POP(A)").errnum(), Some(31));
        assert_eq!(run_b_err("DIM A[0]\nX=SHIFT(A)").errnum(), Some(31));
    }

    /// Read a string global (`A$`) as a Rust `String`.
    fn string(vm: &Vm, ident: &str) -> String {
        match vm.global_value(ident, Suffix::Str).expect("var exists") {
            Value::Str(u) => String::from_utf16_lossy(&u),
            other => panic!("{ident}$ is not Str: {other:?}"),
        }
    }

    fn int(vm: &Vm, name: &str) -> i32 {
        match vm.global_value(name, Suffix::None).expect("var exists") {
            Value::Int(i) => i,
            other => panic!("{name} is not Int: {other:?}"),
        }
    }

    fn real(vm: &Vm, name: &str) -> f64 {
        match vm.global_value(name, Suffix::None).expect("var exists") {
            Value::Real(r) => r,
            other => panic!("{name} is not Real: {other:?}"),
        }
    }

    /// Read a `%`-suffixed (Integer) global by its base name.
    fn int_s(vm: &Vm, name: &str) -> i32 {
        match vm.global_value(name, Suffix::Int).expect("var exists") {
            Value::Int(i) => i,
            other => panic!("{name}% is not Int: {other:?}"),
        }
    }

    // ---- arithmetic & precedence ----

    #[test]
    fn arithmetic_with_precedence() {
        // Folded at parse time, but exercises Push/PopVar.
        let vm = run("A=2+3*4");
        assert_eq!(int(&vm, "A"), 14);
    }

    #[test]
    fn runtime_operators_via_variables() {
        // Not folded (variable operands) -> exercises Operate at runtime.
        let vm = run("A=3\nB=4\nC=A+B*2\nD=A*A-B");
        assert_eq!(int(&vm, "C"), 11);
        assert_eq!(int(&vm, "D"), 5);
    }

    #[test]
    fn suffixless_arithmetic_promotes_on_overflow() {
        // hw_verified (sb-oracle 2026-06-24): a suffix-less (dynamic "Number") operand
        // PROMOTES Int→Double on i32 overflow — it does NOT wrap mod 2^32.
        let vm = run("A=2000000000\nB=A*2");
        assert_eq!(real(&vm, "B"), 4_000_000_000.0); // not wrapping_mul's -294967296
        let vm = run("A=2147483647\nA=A+1");
        assert_eq!(real(&vm, "A"), 2_147_483_648.0);
        let vm = run("A=-2000000000\nB=A-2000000000");
        assert_eq!(real(&vm, "B"), -4_000_000_000.0);
        // A result that still fits i32 stays Integer.
        let vm = run("A=1000000000\nB=A+1");
        assert_eq!(int(&vm, "B"), 1_000_000_001);
    }

    #[test]
    fn integer_typed_arithmetic_wraps() {
        // hw_verified (sb-oracle 2026-06-24): a `%`-typed (statically Integer) operand
        // wraps mod 2^32 on overflow (A%=2000000000:A%*2 → -294967296).
        let vm = run("A%=2000000000\nA%=A%*2");
        let Value::Int(v) = vm.global_value("A", Suffix::Int).expect("A% exists") else {
            panic!("A% not Int");
        };
        assert_eq!(v, -294967296);
    }

    #[test]
    fn inc_dec_follow_scalar_overflow_rule() {
        // hw_verified (sb-oracle 2026-06-24): `INC X`/`DEC X` are `X = X +/- delta`, so a
        // suffix-less (Number) target PROMOTES Int→Double on i32 overflow while a `%`
        // target WRAPS mod 2^32.
        let vm = run("A=2147483647\nINC A");
        assert_eq!(real(&vm, "A"), 2_147_483_648.0);
        let vm = run("A=-2147483648\nDEC A");
        assert_eq!(real(&vm, "A"), -2_147_483_649.0);
        let vm = run("A=2147483640\nINC A,10");
        assert_eq!(real(&vm, "A"), 2_147_483_650.0);
        // `%` target wraps (and a result still in range stays Integer).
        let vm = run("A%=2147483647\nINC A%");
        assert_eq!(int_s(&vm, "A"), -2147483648);
        let vm = run("A%=-2147483648\nDEC A%");
        assert_eq!(int_s(&vm, "A"), 2147483647);
    }

    #[test]
    fn for_counter_promotes_on_overflow_and_terminates() {
        // A suffix-less FOR counter that overruns i32 PROMOTES Int→Double on the step add,
        // so the loop terminates instead of wrapping into an endless loop. Derived from the
        // hw_verified INC promotion; FOR-specific oracle confirm queued (bd:sb-interpreter-air).
        // STEP 2e9 from 2e9: body sees I=2e9, then I=4e9 (promoted), then I=6e9 > TO → stop.
        let vm = run("N=0\nFOR I=2000000000 TO 4500000000 STEP 2000000000\nN=I\nNEXT");
        assert_eq!(real(&vm, "N"), 4_000_000_000.0);
        // A `%` counter keeps the wrapping add; a small in-range loop terminates normally.
        let vm = run("FOR I%=1 TO 3\nNEXT");
        assert_eq!(int_s(&vm, "I"), 4);
    }

    #[test]
    fn heavy_program_is_deterministic() {
        // M7-T5 perf/determinism guard: a heavy arithmetic+loop workload (~100k
        // iterations through the promoting add/mul + MOD paths) runs to completion and
        // yields byte-identical results across runs (no RNG/clock/IO in core).
        let src = "S=0\nFOR I=1 TO 100000\nS=(S+I*3) MOD 1000000\nNEXT";
        let a = run(src);
        let b = run(src);
        assert_eq!(int(&a, "S"), int(&b, "S"));
        assert!(int(&a, "S") >= 0 && int(&a, "S") < 1000000);
    }

    #[test]
    fn shift_count_is_not_masked() {
        // hw_verified (sb-oracle 2026-06-24): the shift count is not taken mod 32.
        let vm = run(concat!(
            "A=1<<31\nB=1<<30\nC=1<<16\nD=1<<32\nE=1<<33\nF=3<<1\nG=-1<<1\n",
            "H=256>>2\nI=-1>>1\nJ=-2>>1\nK=-2147483648>>1\nL=1>>1\n",
            "M=-1>>32\nN=5>>32\nO=2<<-1\nP=256>>-1"
        ));
        assert_eq!(int(&vm, "A"), -2147483648); // 1<<31 sets the sign bit
        assert_eq!(int(&vm, "B"), 1073741824);
        assert_eq!(int(&vm, "C"), 65536);
        assert_eq!(int(&vm, "D"), 0); // 1<<32 → 0 (not 1<<0=1)
        assert_eq!(int(&vm, "E"), 0); // 1<<33 → 0 (not 1<<1=2)
        assert_eq!(int(&vm, "F"), 6);
        assert_eq!(int(&vm, "G"), -2);
        assert_eq!(int(&vm, "H"), 64);
        assert_eq!(int(&vm, "I"), -1); // >> is arithmetic (sign-extending)
        assert_eq!(int(&vm, "J"), -1);
        assert_eq!(int(&vm, "K"), -1073741824);
        assert_eq!(int(&vm, "L"), 0);
        assert_eq!(int(&vm, "M"), -1); // -1>>32 saturates to the sign bit
        assert_eq!(int(&vm, "N"), 0); // 5>>32 → 0
        assert_eq!(int(&vm, "O"), 0); // negative count → 0
        assert_eq!(int(&vm, "P"), 0);
    }

    #[test]
    fn out_of_range_double_to_int_store_is_overflow() {
        // hw_verified (sb-oracle 2026-06-24): storing a Double outside the i32 range into
        // a `%` variable (or `%` array element) raises Overflow (errnum 9); the guard is
        // on the value, so a fractional value just past the limit overflows too.
        // (decimal-form huge literals — the lexer promotes >i32 integer literals to
        // Double; E-notation is a separate queued lexer gap.)
        assert_eq!(run_err("A%=10000000000").errnum(), Some(9)); // 1e10
        assert_eq!(run_err("A%=2147483648.0").errnum(), Some(9));
        assert_eq!(run_err("A%=2147483647.9").errnum(), Some(9));
        assert_eq!(run_err("A%=-2147483648.9").errnum(), Some(9));
        assert_eq!(run_err("DIM A%[1]\nA%[0]=10000000000").errnum(), Some(9));
        // In-range values truncate toward zero without error.
        let vm = run("A%=2147483647.0\nB%=-2147483648.0\nC%=2147483646.9\nD%=2.7");
        assert_eq!(int_s(&vm, "A"), 2147483647);
        assert_eq!(int_s(&vm, "B"), -2147483648);
        assert_eq!(int_s(&vm, "C"), 2147483646);
        assert_eq!(int_s(&vm, "D"), 2);
    }

    #[test]
    fn slash_is_real_division() {
        let vm = run("A=7\nB=2\nC=A/B");
        assert_eq!(real(&vm, "C"), 3.5);
    }

    #[test]
    fn div_mod_truncate_operands_first() {
        // 7 DIV 2 == 3, -7 DIV 2 == -3 (toward zero); 7 MOD 3 == 1, -7 MOD 3 == -1.
        let vm = run("A=7 DIV 2\nB=-7 DIV 2\nC=7 MOD 3\nD=-7 MOD 3");
        assert_eq!(int(&vm, "A"), 3);
        assert_eq!(int(&vm, "B"), -3);
        assert_eq!(int(&vm, "C"), 1);
        assert_eq!(int(&vm, "D"), -1);
    }

    #[test]
    fn bitwise_truncates_doubles() {
        // 7 AND 2.9 == 2 (operand truncation toward zero, not rounding).
        let vm = run("X=2.9\nA=7 AND X\nB=128 OR &HA3\nC=100 XOR &H4C");
        assert_eq!(int(&vm, "A"), 2);
        assert_eq!(int(&vm, "B"), 163);
        assert_eq!(int(&vm, "C"), 40);
    }

    #[test]
    fn divide_by_zero_is_errnum_7() {
        assert_eq!(run_err("A=1\nB=A/0").errnum(), Some(7));
        assert_eq!(run_err("A=1\nB=A DIV 0").errnum(), Some(7));
        assert_eq!(run_err("A=1\nB=A MOD 0").errnum(), Some(7));
    }

    #[test]
    fn string_in_arithmetic_is_type_mismatch() {
        assert_eq!(
            run_err(
                r#"A$="x"
B=A$+1"#
            )
            .errnum(),
            Some(8)
        );
    }

    #[test]
    fn string_concatenation() {
        let vm = run(r#"A$="foo"
B$="bar"
C$=A$+B$"#);
        match vm.global_value("C", Suffix::Str).unwrap() {
            Value::Str(s) => assert_eq!(String::from_utf16_lossy(&s), "foobar"),
            other => panic!("not a string: {other:?}"),
        }
    }

    #[test]
    fn string_repetition() {
        // `"A"*3` and `3*"AB"` repeat the string; a non-positive count yields ""
        // (behavior uncovered by the FONTDEF conformance cases).
        assert_eq!(out(r#"PRINT "A"*3"#), "AAA");
        assert_eq!(out(r#"PRINT 3*"AB""#), "ABABAB");
        assert_eq!(out(r#"PRINT "X"*0"#), "");
    }

    #[test]
    fn comparisons_yield_one_or_zero() {
        let vm = run("A=3\nB=4\nLT=A<B\nGE=A>=B\nEQ=A==3");
        assert_eq!(int(&vm, "LT"), 1);
        assert_eq!(int(&vm, "GE"), 0);
        assert_eq!(int(&vm, "EQ"), 1);
    }

    #[test]
    fn unary_neg_and_not() {
        let vm = run("A=5\nB=-A\nC=NOT 0\nD=!A");
        assert_eq!(int(&vm, "B"), -5);
        assert_eq!(int(&vm, "C"), !0); // bitwise complement of 0 = -1
        assert_eq!(int(&vm, "D"), 0); // logical not of nonzero
    }

    // ---- coercion on assignment ----

    #[test]
    fn integer_suffix_truncates_toward_zero() {
        // A%=2.7 -> 2 (hw_verified coercion, exercised through PopVar).
        let vm = run("A%=2.7\nB%=-2.7");
        assert_eq!(vm.global_value("A", Suffix::Int).unwrap(), Value::Int(2));
        assert_eq!(vm.global_value("B", Suffix::Int).unwrap(), Value::Int(-2));
    }

    // ---- control flow ----

    #[test]
    fn if_then_else() {
        let vm = run("A=5\nIF A>3 THEN B=1 ELSE B=2");
        assert_eq!(int(&vm, "B"), 1);
        let vm = run("A=1\nIF A>3 THEN B=1 ELSE B=2");
        assert_eq!(int(&vm, "B"), 2);
    }

    #[test]
    fn for_loop_sums() {
        let vm = run("S=0\nFOR I=1 TO 10\nS=S+I\nNEXT");
        assert_eq!(int(&vm, "S"), 55);
    }

    #[test]
    fn for_loop_negative_step() {
        let vm = run("S=0\nFOR I=5 TO 1 STEP -1\nS=S+I\nNEXT");
        assert_eq!(int(&vm, "S"), 15);
        assert_eq!(int(&vm, "I"), 0); // last decrement steps past the bound
    }

    #[test]
    fn while_loop() {
        let vm = run("I=0\nS=0\nWHILE I<5\nS=S+I\nI=I+1\nWEND");
        assert_eq!(int(&vm, "S"), 10);
    }

    #[test]
    fn repeat_until() {
        let vm = run("I=0\nS=0\nREPEAT\nS=S+I\nI=I+1\nUNTIL I>=5");
        assert_eq!(int(&vm, "S"), 10);
    }

    #[test]
    fn break_and_continue() {
        // sum 1..10 but skip 5 and stop at 8.
        let vm = run("S=0\nFOR I=1 TO 10\nIF I==5 THEN CONTINUE\nIF I>8 THEN BREAK\nS=S+I\nNEXT");
        // 1+2+3+4+6+7+8 = 31
        assert_eq!(int(&vm, "S"), 31);
    }

    #[test]
    fn short_circuit_and_or() {
        let vm = run("A=1\nB=0\nX=A&&B\nY=A||B");
        assert_eq!(int(&vm, "X"), 0);
        assert_eq!(int(&vm, "Y"), 1);
    }

    // ---- GOSUB / ON ----

    #[test]
    fn gosub_return() {
        let vm = run("A=0\nGOSUB @SUB\nA=A+10\nEND\n@SUB\nA=5\nRETURN");
        assert_eq!(int(&vm, "A"), 15);
    }

    #[test]
    fn return_without_gosub_is_errnum_30() {
        assert_eq!(run_err("RETURN").errnum(), Some(30));
    }

    #[test]
    fn on_goto_selects_branch() {
        // index 1 selects the second target (0-based).
        let vm = run("N=1\nR=0\nON N GOTO @A,@B\n@A\nR=10\nEND\n@B\nR=20\nEND");
        assert_eq!(int(&vm, "R"), 20);
    }

    #[test]
    fn on_goto_out_of_range_falls_through() {
        let vm = run("N=5\nR=0\nON N GOTO @A,@B\nR=99\nEND\n@A\nR=10\nEND\n@B\nR=20\nEND");
        assert_eq!(int(&vm, "R"), 99);
    }

    #[test]
    fn on_gosub_returns() {
        let vm = run("N=0\nR=0\nON N GOSUB @A\nR=R+1\nEND\n@A\nR=100\nRETURN");
        assert_eq!(int(&vm, "R"), 101);
    }

    // ---- arrays ----

    #[test]
    fn array_declare_store_read() {
        let vm = run("DIM A[5]\nA[2]=7\nB=A[2]");
        assert_eq!(real(&vm, "B"), 7.0);
    }

    #[test]
    fn array_2d_row_major() {
        let vm = run("DIM P[3,2]\nP[2,1]=9\nB=P[2,1]");
        assert_eq!(real(&vm, "B"), 9.0);
    }

    #[test]
    fn array_subscript_out_of_range_is_errnum_31() {
        assert_eq!(run_err("DIM A[3]\nB=A[3]").errnum(), Some(31));
    }

    // ---- DATA / READ / RESTORE ----

    #[test]
    fn data_read_restore() {
        let vm = run("READ A,B\nRESTORE @D\nREAD C\nDATA 10,20,30\n@D\nDATA 99");
        assert_eq!(int(&vm, "A"), 10);
        assert_eq!(int(&vm, "B"), 20);
        assert_eq!(int(&vm, "C"), 99);
    }

    #[test]
    fn out_of_data_is_errnum_13() {
        assert_eq!(run_err("READ A,B\nDATA 1").errnum(), Some(13));
    }

    #[test]
    fn bare_restore_is_type_mismatch_8() {
        // Argument-less RESTORE has no reset-to-first form on real SB 3.6.0; it
        // raises Type mismatch (8) at runtime — hw_verified (restore.yaml, oracle
        // 2026-06-23: bare `RESTORE` halts with errnum 8 where `RESTORE @D1` works).
        let e = run_err("READ A:RESTORE:READ B:@D1:DATA 7");
        assert_eq!(e.errnum(), Some(8));
        assert_eq!(e.errline(), Some(1));
    }

    // ---- INC / SWAP ----

    #[test]
    fn inc_and_dec() {
        let vm = run("A=5\nINC A\nINC A,3\nDEC A,2");
        assert_eq!(int(&vm, "A"), 7);
    }

    #[test]
    fn swap_scalars() {
        let vm = run("A=1\nB=2\nSWAP A,B");
        assert_eq!(int(&vm, "A"), 2);
        assert_eq!(int(&vm, "B"), 1);
    }

    #[test]
    fn swap_typed_var_truncates_to_declared_type() {
        // The otya SWAPTEST case: `SWAP A%,B#` re-coerces each value to its
        // destination's declared suffix. A% (Integer) truncates B#'s 2.34567 → 2;
        // B# (Real) widens A%'s 1 → 1.0. (swap.yaml hw_verified coercion rule.)
        let vm = run("A%=1\nB#=2.34567\nSWAP A%,B#");
        match vm.global_value("A", Suffix::Int).expect("A% exists") {
            Value::Int(i) => assert_eq!(i, 2),
            other => panic!("A% is not Int: {other:?}"),
        }
        match vm.global_value("B", Suffix::Real).expect("B# exists") {
            Value::Real(r) => assert_eq!(r, 1.0),
            other => panic!("B# is not Real: {other:?}"),
        }
    }

    #[test]
    fn swap_untyped_var_keeps_value_verbatim() {
        // An untyped numeric var takes the swapped value VERBATIM (no truncation):
        // `SWAP A,B#` leaves A holding the Double 2.5, B# holding 1.0.
        // (swap.yaml hw_verified 2026-06-22.)
        let vm = run("A=1\nB#=2.5\nSWAP A,B#");
        assert_eq!(real(&vm, "A"), 2.5);
        match vm.global_value("B", Suffix::Real).expect("B# exists") {
            Value::Real(r) => assert_eq!(r, 1.0),
            other => panic!("B# is not Real: {other:?}"),
        }
    }

    #[test]
    fn swap_string_numeric_is_type_mismatch_8() {
        // Mixing a string and a numeric operand → Type mismatch (8). The coerce
        // happens before either cell is written, so both stay untouched.
        let e = run_err("A=1\nB$=\"x\"\nSWAP A,B$");
        assert_eq!(e.errnum(), Some(8));
    }

    // ---- array-element references (Op::PushArrayRef, M1-T14) ----

    #[test]
    fn swap_array_elements() {
        // SWAP A[0],A[2] exchanges two elements of the SAME array (the ref shares
        // the array Rc). swap.yaml hw_verified: (10,99) -> (99,10).
        let vm = run("DIM A[3]\nA[0]=10\nA[2]=99\nSWAP A[0],A[2]\nPRINT A[0];A[2]");
        assert_eq!(vm.console_text(), "9910");
    }

    #[test]
    fn swap_scalar_and_array_element() {
        // Mixed scalar/array-element operands are legal. swap.yaml hw_verified:
        // X=5,B[1]=9 -> X=9,B[1]=5.
        let vm = run("X=5\nDIM B[3]\nB[1]=9\nSWAP X,B[1]\nPRINT X;B[1]");
        assert_eq!(vm.console_text(), "95");
    }

    #[test]
    fn swap_aliased_array_element_is_noop() {
        // SWAP A[1],A[1] reads both before writing either, so the element is
        // unchanged (an aliased SWAP collapses to a no-op).
        let vm = run("DIM A[3]\nA[1]=7\nSWAP A[1],A[1]\nPRINT A[1]");
        assert_eq!(vm.console_text(), "7");
    }

    #[test]
    fn swap_2d_array_elements() {
        // Multi-dimensional element refs: each subscript tuple resolves to a flat
        // offset (swap.yaml: SWAP A[X1,Y1],A[X2,Y2] is legal).
        let vm = run("DIM A[2,2]\nA[0,0]=1\nA[1,1]=2\nSWAP A[0,0],A[1,1]\nPRINT A[0,0];A[1,1]");
        assert_eq!(vm.console_text(), "21");
    }

    #[test]
    fn swap_string_array_elements() {
        // String-array element refs go through the Str element type.
        let vm = run("DIM S$[3]\nS$[0]=\"A\"\nS$[2]=\"B\"\nSWAP S$[0],S$[2]\nPRINT S$[0];S$[2]");
        assert_eq!(vm.console_text(), "BA");
    }

    #[test]
    fn swap_array_element_no_suffix_is_real() {
        // An unsuffixed array is Real by default: swapping a Double in keeps it as 2.7.
        // The untyped scalar X takes 10 verbatim.
        let vm = run("DIM A[2]\nA[0]=10\nX=2.7\nSWAP A[0],X\nPRINT A[0];\",\";X");
        assert_eq!(vm.console_text(), "2.7,10");
    }

    #[test]
    fn inc_array_element() {
        // INC A[1],5 increments through an element ref. inc.yaml hw_verified: 10 -> 15.
        let vm = run("DIM A[3]\nA[1]=10\nINC A[1],5\nPRINT A[1]");
        assert_eq!(vm.console_text(), "15");
    }

    #[test]
    fn dec_array_element() {
        let vm = run("DIM A[3]\nA[1]=10\nDEC A[1],3\nPRINT A[1]");
        assert_eq!(vm.console_text(), "7");
    }

    #[test]
    fn array_element_ref_out_of_range_is_31() {
        // Taking a ref to an out-of-range element bounds-checks at ref time:
        // Subscript out of range (errnum 31), like a plain element read.
        let e = run_err("DIM A[3]\nSWAP A[0],A[5]");
        assert_eq!(e.errnum(), Some(31));
    }

    #[test]
    fn swap_string_array_element_with_numeric_scalar_is_type_mismatch_8() {
        // Cross-type (string element ↔ numeric scalar) → Type mismatch (8), caught
        // before either target is written.
        let e = run_err("DIM S$[2]\nS$[0]=\"x\"\nN=1\nSWAP S$[0],N");
        assert_eq!(e.errnum(), Some(8));
    }

    // ---- DEF functions ----

    #[test]
    fn def_value_function() {
        let vm = run("DEF SQ(X)\nRETURN X*X\nEND\nA=SQ(5)");
        assert_eq!(int(&vm, "A"), 25);
    }

    #[test]
    fn def_with_out_param() {
        let vm = run("DEF HALF X OUT Y\nY=X/2\nEND\nHALF 10 OUT R");
        assert_eq!(real(&vm, "R"), 5.0);
    }

    #[test]
    fn def_recursion_factorial() {
        let vm = run("DEF FACT(N)\nIF N<=1 THEN RETURN 1\nRETURN N*FACT(N-1)\nEND\nA=FACT(5)");
        assert_eq!(int(&vm, "A"), 120);
    }

    // ---- CALL "name" — dynamic dispatch (M6-T6) ----

    #[test]
    fn call_by_name_runs_the_def() {
        // hw_verified (call.yaml calls_by_name): CALL "GREET" invokes DEF GREET.
        let vm = run(r#"DEF GREET
PRINT "HI"
END
CALL "GREET""#);
        assert_eq!(vm.console_text(), "HI");
    }

    #[test]
    fn call_undefined_name_is_undefined_function() {
        // hw_verified (call.yaml undefined_instruction): an unknown name → errnum 16.
        let err = run_err(r#"CALL "NOPE""#);
        assert_eq!(err.errnum(), Some(16));
    }

    #[test]
    fn call_name_is_case_insensitive() {
        // Names fold ASCII to uppercase, so a lowercase CALL string still resolves.
        let vm = run(r#"DEF GREET
PRINT "HI"
END
CALL "greet""#);
        assert_eq!(vm.console_text(), "HI");
    }

    #[test]
    fn call_name_from_a_string_variable() {
        // The target is chosen at runtime — a string variable selects the DEF.
        let vm = run(r#"N$="GREET"
DEF GREET
PRINT "HI"
END
CALL N$"#);
        assert_eq!(vm.console_text(), "HI");
    }

    #[test]
    fn call_passes_value_args() {
        let vm = run(r#"DEF ADD A,B
PRINT A+B
END
CALL "ADD",2,3"#);
        assert_eq!(vm.console_text(), "5");
    }

    #[test]
    fn call_returns_out_args() {
        let vm = run(r#"DEF ADDOUT A,B OUT R
R=A+B
END
CALL "ADDOUT",2,3 OUT X"#);
        assert_eq!(int(&vm, "X"), 5);
    }

    #[test]
    fn call_with_a_non_string_name_is_type_mismatch() {
        // The name operand must be a string; a numeric one → Type mismatch (8).
        let err = run_err("CALL 5");
        assert_eq!(err.errnum(), Some(8));
    }

    // ---- cross-slot COMMON DEF dispatch (M6-T6) ----

    /// Compile a snippet (with the builtin registry) into a loadable slot program.
    fn slot_program(src: &str) -> Program {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse slot");
        compile_with(&ast, &StdBuiltins).expect("compile slot")
    }

    /// Build the running (slot-0) program, load `src1` into slot 1, run, return the VM.
    fn run_with_slot1(slot0: &str, slot1: &str) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(slot0).expect("parse slot0");
        let program = compile_with(&ast, &StdBuiltins).expect("compile slot0");
        let mut vm = Vm::new(program);
        vm.load_slot_program(1, slot_program(slot1));
        vm.run().expect("run");
        vm
    }

    #[test]
    fn common_def_is_callable_cross_slot_after_use() {
        // documented (common.yaml): with COMMON a DEF is callable from another slot once
        // that slot is USE'd; CALL "name" (call.yaml) is the by-name dispatch.
        let vm = run_with_slot1(
            "USE 1\nCALL \"SHOUT\"",
            "COMMON DEF SHOUT\nPRINT \"HEY\"\nEND",
        );
        assert_eq!(vm.console_text(), "HEY");
    }

    #[test]
    fn cross_slot_common_def_passes_args_and_out() {
        // The call/return semantics are those of DEF (common.yaml): value args bind, OUT
        // results come back — into the *caller's* slot-0 variable.
        let vm = run_with_slot1(
            "USE 1\nCALL \"ADD3\",10,20 OUT R\nPRINT R",
            "COMMON DEF ADD3 A,B OUT C\nC=A+B+3\nEND",
        );
        assert_eq!(vm.console_text(), "33");
    }

    #[test]
    fn cross_slot_call_without_use_is_undefined() {
        // USE is required (common.yaml): a loaded-but-not-USE'd slot's COMMON DEF does not
        // resolve → Undefined function (16).
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("CALL \"SHOUT\"").expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.load_slot_program(1, slot_program("COMMON DEF SHOUT\nPRINT \"HEY\"\nEND"));
        let err = vm.run().expect_err("must be undefined without USE");
        assert_eq!(err.errnum(), Some(16));
    }

    #[test]
    fn cross_slot_non_common_def_is_private() {
        // Without COMMON a DEF is private to its own slot (common.yaml): even a USE'd slot's
        // plain DEF is not visible cross-slot → Undefined function (16).
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("USE 1\nCALL \"PRIV\"").expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.load_slot_program(1, slot_program("DEF PRIV\nPRINT \"NO\"\nEND"));
        let err = vm
            .run()
            .expect_err("private DEF must not resolve cross-slot");
        assert_eq!(err.errnum(), Some(16));
    }

    #[test]
    fn cross_slot_call_restores_caller_context() {
        // After the cross-slot COMMON DEF returns, execution resumes in slot 0 against its
        // own globals — a following slot-0 statement still sees slot-0 state.
        let vm = run_with_slot1(
            "X=7\nUSE 1\nCALL \"NOTE\"\nPRINT X",
            "COMMON DEF NOTE\nPRINT \"IN\";\nEND",
        );
        // slot-1's COMMON DEF printed "IN" (no newline, trailing `;`); resuming in slot 0,
        // `PRINT X` still sees slot-0's X=7 → "IN7".
        assert_eq!(vm.console_text(), "IN7");
    }

    #[test]
    fn cross_slot_common_def_can_return_a_value() {
        // A COMMON DEF returns values to its caller via the CALL statement's OUT clause
        // (real SB 3.6.0 has NO function form `V=CALL(...)` — that raises Syntax error 3;
        // see `call_function_form_is_syntax_error`). The OUT form resolves cross-slot.
        let vm = run_with_slot1(
            "USE 1\nCALL \"DBL\",21 OUT V\nPRINT V",
            "COMMON DEF DBL N OUT R\nR=N*2\nEND",
        );
        assert_eq!(vm.console_text(), "42");
    }

    #[test]
    fn call_function_form_is_syntax_error() {
        // `V=CALL("F",args)` (CALL used as a value-returning function expression) is a
        // Syntax error (errnum 3) at compile — real SB 3.6.0 has no function form of
        // CALL (hw_verified, s_t3e harvest 2026-06-26, both same- and cross-slot). A
        // value returns to the caller only via the OUT clause of the statement form.
        let ast = parse("V=CALL(\"DBL\",21)").expect("parse");
        let err = crate::compiler::compile(&ast).expect_err("expected a compile error");
        assert_eq!(err.errnum, 3, "CALL function form should be Syntax error");
    }

    #[test]
    fn cross_slot_common_def_global_binds_to_defining_slot() {
        // A global referenced inside a COMMON DEF binds to the DEFINING slot's globals, not
        // the caller's — even when both slots declare a same-named global. Two osb rules
        // compose: (1) compiler.d:318-345 — a name inside a DEF resolves to a GLOBAL only
        // when that global already exists in the slot's table (here slot 1's top-level `G=0`
        // registers G as a slot-1 global; without it G would be a fresh DEF-local), and
        // (2) VM.d:585-589 — a COMMON-function call does `setCurrentSlot(func.slot.index)`,
        // so that global read resolves against the defining slot (`currentSlot.global[...]`).
        // Slot 0 sets G=7; slot 1's top-level `G=0` does NOT run on a CALL (USE only), so
        // slot 1's G is the load-time zero-init 0. Defining-slot scoping → SHOWG prints slot
        // 1's G (0), not the caller's 7.
        let vm = run_with_slot1(
            "G=7\nUSE 1\nCALL \"SHOWG\"",
            "G=0\nCOMMON DEF SHOWG\nPRINT G\nEND",
        );
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn cross_slot_common_def_global_write_isolated_from_caller() {
        // The write direction of the same rule: a global assigned inside a COMMON DEF lands
        // in the DEFINING slot's globals, leaving the caller's same-named global untouched.
        // Slot 1 declares G as a global (top-level `G=0`); `SETG` writes G=99 into slot 1's
        // globals, `SHOWG` reads it back (sees slot 1's G=99 → "99|"), then slot 0's
        // `PRINT G` shows its own G still 7.
        let vm = run_with_slot1(
            "G=7\nUSE 1\nCALL \"SETG\"\nCALL \"SHOWG\"\nPRINT G",
            "G=0\nCOMMON DEF SETG\nG=99\nEND\nCOMMON DEF SHOWG\nPRINT G;\"|\";\nEND",
        );
        assert_eq!(vm.console_text(), "99|7");
    }

    // ---- EXEC numeric control transfer (M6-T6) ----

    #[test]
    fn exec_numeric_transfers_to_loaded_slot() {
        // documented (exec.yaml): `EXEC n` executes the program already loaded in slot n.
        // Control transfers to slot 1 ("B"); then per the documented cross-slot return rule
        // ("It is possible to return by using END in a program started with EXEC in another
        // SLOT") slot 1's END returns to the launcher, resuming right after the EXEC → "C".
        let vm = run_with_slot1("PRINT \"A\";\nEXEC 1\nPRINT \"C\"", "PRINT \"B\";\nEND");
        assert_eq!(vm.console_text(), "ABC");
    }

    #[test]
    fn exec_cross_slot_end_returns_to_launcher() {
        // The documented END-returns-across-slots rule (exec.md): a program EXEC'd into a
        // DIFFERENT slot returns to its launcher when it hits END. The launcher resumes with
        // its own slot-0 state intact — `X` set before the EXEC is still readable afterward.
        let vm = run_with_slot1("X=7\nEXEC 1\nPRINT \"BACK\";X", "PRINT \"IN1\";\nEND");
        assert_eq!(vm.console_text(), "IN1BACK7");
    }

    #[test]
    fn exec_cross_slot_nesting_returns_in_lifo_order() {
        // Nesting: slot 0 EXECs slot 1, which EXECs slot 2. Slot 2's END returns to slot 1,
        // whose END returns to slot 0 — LIFO, each launcher resuming after its own EXEC.
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("PRINT \"0A\";\nEXEC 1\nPRINT \"0B\";").expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.load_slot_program(1, slot_program("PRINT \"1A\";\nEXEC 2\nPRINT \"1B\";\nEND"));
        vm.load_slot_program(2, slot_program("PRINT \"2\";\nEND"));
        vm.run().expect("run");
        assert_eq!(vm.console_text(), "0A1A21B0B");
    }

    #[test]
    fn exec_transfer_runs_target_against_its_own_globals() {
        // The EXEC'd program runs with the target slot's own globals, not the caller's.
        // Slot 0 sets X=99 but never uses it; slot 1 has its own X defaulting to 0.
        let vm = run_with_slot1("X=99\nEXEC 1", "PRINT X\nEND");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn exec_unloaded_non_running_slot_is_syntax_error() {
        // hw_verified (exec.yaml): `EXEC 1` on an empty (no program loaded) non-running slot
        // → Syntax error (3) — the transfer only fires when a program is loaded there.
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("EXEC 1").expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        let err = vm.run().expect_err("empty slot must be Syntax error");
        assert_eq!(err.errnum(), Some(3));
    }

    #[test]
    fn exec_cross_slot_return_preserves_caller_gosub_state() {
        // The launcher's GOSUB stack is captured across a cross-slot EXEC and restored on the
        // END-return: EXEC fired from inside a GOSUB, so after slot 1's END resumes the caller
        // its `RETURN` still pops back to the line after `GOSUB @SUB` → "AFTER".
        let vm = run_with_slot1(
            "GOSUB @SUB\nPRINT \"AFTER\"\nSTOP\n@SUB\nPRINT \"IN0\";\nEXEC 1\nRETURN",
            "PRINT \"IN1\";\nEND",
        );
        assert_eq!(vm.console_text(), "IN0IN1AFTER");
    }

    #[test]
    fn exec_transfer_error_reports_target_slot() {
        // After the transfer, an error in the EXEC'd program reports ERRPRG = its slot.
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("EXEC 1").expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        vm.load_slot_program(1, slot_program("A=SQR(-1)\nEND"));
        let _ = vm.run();
        assert_eq!(vm.errnum(), 10);
        assert_eq!(vm.errprg(), 1);
    }

    // ---- EXEC / USE string-form file LOAD (M6-T6) ----

    /// Build a slot-0 program, seed `files` (name → source) into the VM's `MemStorage`
    /// (project `DEFAULT`, TXT folder), run it, and return the result.
    fn run_with_txt(slot0: &str, files: &[(&str, &str)]) -> Result<Vm, VmError> {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        use crate::storage::{MemStorage, Storage, DEFAULT_PROJECT};
        let mut store = MemStorage::default();
        for (name, body) in files {
            store
                .write(DEFAULT_PROJECT, Folder::Txt, name, body.as_bytes())
                .expect("seed txt");
        }
        let ast = parse(slot0).expect("parse slot0");
        let program = compile_with(&ast, &StdBuiltins).expect("compile slot0");
        let mut vm = Vm::new(program);
        vm.set_storage(Box::new(store));
        vm.run()?;
        Ok(vm)
    }

    #[test]
    fn exec_string_loads_prg_slot_and_transfers_control() {
        // documented (exec.yaml form 1): `EXEC "PRGn:file"` loads the file's program into
        // slot n and runs it. Control transfers to the loaded program ("B"); since the load
        // targets a DIFFERENT slot, its END returns to the launcher, resuming after EXEC → "C".
        let vm = run_with_txt(
            "PRINT \"A\";\nEXEC \"PRG1:SUB\"\nPRINT \"C\"",
            &[("SUB", "PRINT \"B\";\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "ABC");
    }

    #[test]
    fn exec_string_runs_loaded_program_against_its_own_globals() {
        // The EXEC'd program runs with the loaded slot's own globals — slot 0's X is not
        // visible (matches the numeric-transfer scoping).
        let vm = run_with_txt("X=99\nEXEC \"PRG2:SUB\"", &[("SUB", "PRINT X\nEND")]).expect("run");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn exec_string_missing_file_is_load_failed() {
        // hw_verified (exec.yaml): a `PRGn:` resource naming an absent file → Load failed (46).
        let err = run_with_txt("EXEC \"PRG1:NOPE\"", &[])
            .err()
            .expect("missing file");
        assert_eq!(err.errnum(), Some(46));
    }

    #[test]
    fn exec_string_into_running_slot_loads_and_runs_from_top() {
        // documented (exec.yaml form 1): `EXEC "PRG0:file"` while slot 0 runs loads the file's
        // program INTO the running slot and runs it from the top (the corpus loader idiom).
        let vm = run_with_txt(
            "PRINT \"A\";\nEXEC \"PRG0:SUB\"\nPRINT \"NEVER\"",
            &[("SUB", "PRINT \"B\"\nEND")],
        )
        .expect("run");
        // The slot-0 statement after EXEC is abandoned (can't return to the previous program).
        assert_eq!(vm.console_text(), "AB");
    }

    #[test]
    fn exec_string_running_slot_runs_against_fresh_globals() {
        // The loaded program replaces the running one and starts with its own zeroed globals —
        // slot 0's pre-EXEC X is not visible to the loaded program.
        let vm = run_with_txt("X=99\nEXEC \"PRG0:SUB\"", &[("SUB", "PRINT X\nEND")]).expect("run");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn exec_string_running_slot_missing_file_is_load_failed() {
        // hw_verified (exec.yaml): a `PRG0:` resource naming an absent file → Load failed (46),
        // even when it targets the running slot.
        let err = run_with_txt("EXEC \"PRG0:NOPE\"", &[])
            .err()
            .expect("missing file");
        assert_eq!(err.errnum(), Some(46));
    }

    #[test]
    fn exec_string_bare_name_loads_into_running_slot() {
        // documented + osb structural (exec.yaml): a bare filename (no `PRGn:` resource number)
        // defaults to the running slot, so `EXEC "FILE"` while slot 0 runs loads + runs the file
        // in the running slot from the top (the corpus loader idiom `EXEC EXE$`).
        let vm = run_with_txt(
            "PRINT \"A\";\nEXEC \"SUB\"\nPRINT \"NEVER\"",
            &[("SUB", "PRINT \"B\"\nEND")],
        )
        .expect("run");
        // The slot-0 statement after EXEC is abandoned (can't return to the previous program).
        assert_eq!(vm.console_text(), "AB");
    }

    #[test]
    fn exec_string_bare_name_runs_against_fresh_globals() {
        // The bare-name load behaves like the running-slot `PRG0:` load: the loaded program
        // replaces the running one with its own zeroed globals (slot 0's X is not visible).
        let vm = run_with_txt("X=99\nEXEC \"SUB\"", &[("SUB", "PRINT X\nEND")]).expect("run");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn exec_string_bare_name_from_variable() {
        // The filename is any string expression — the corpus form `EXEC EXE$` (a bare variable).
        let vm = run_with_txt(
            "EXE$=\"SUB\"\nEXEC EXE$",
            &[("SUB", "PRINT \"LOADED\"\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "LOADED");
    }

    /// Like [`run_with_txt`] but also seeds slot 1 with `slot1`'s program, so a test can make
    /// slot 0 *non-running* (boot slot 0 `EXEC 1` transfers to slot 1) before exercising a
    /// `PRG0:`-into-non-running-slot-0 load from slot 1.
    fn run_slot1_with_txt(slot0: &str, slot1: &str, files: &[(&str, &str)]) -> Result<Vm, VmError> {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        use crate::storage::{MemStorage, Storage, DEFAULT_PROJECT};
        let mut store = MemStorage::default();
        for (name, body) in files {
            store
                .write(DEFAULT_PROJECT, Folder::Txt, name, body.as_bytes())
                .expect("seed txt");
        }
        let ast = parse(slot0).expect("parse slot0");
        let program = compile_with(&ast, &StdBuiltins).expect("compile slot0");
        let mut vm = Vm::new(program);
        vm.set_storage(Box::new(store));
        vm.load_slot_program(1, slot_program(slot1));
        vm.run()?;
        Ok(vm)
    }

    #[test]
    fn exec_string_into_non_running_slot_0() {
        // The slot-0 registry edge (M6-T6): a `PRG0:` resource naming a *non-running* slot 0
        // loads the file into slot 0 and transfers control to it, uniform with any other slot
        // (osb keeps all program SLOTs in one `slots[]` array — slot 0 is not special). Boot
        // slot 0 EXECs slot 1; from slot 1, `EXEC "PRG0:OTHER"` finds slot 0 non-running, loads
        // OTHER into it, and runs it. Being a DIFFERENT slot from the running slot 1, OTHER's
        // END returns to its slot-1 launcher (the documented cross-slot return) → "BACK1";
        // slot 1 then STOPs (a genuine halt, no further return).
        let vm = run_slot1_with_txt(
            "EXEC 1",
            "EXEC \"PRG0:OTHER\"\nPRINT \"BACK1\";\nSTOP",
            &[("OTHER", "PRINT \"IN0\";\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "IN0BACK1");
    }

    #[test]
    fn exec_string_non_running_slot_0_runs_against_its_own_globals() {
        // OTHER loaded into slot 0 runs against slot 0's own (freshly zeroed) globals — slot 1's
        // X=11 is not visible (matches the other cross-slot transfers' scoping).
        let vm = run_slot1_with_txt(
            "EXEC 1",
            "X=11\nEXEC \"PRG0:OTHER\"\nSTOP",
            &[("OTHER", "PRINT X;\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn exec_string_non_running_slot_0_missing_file_is_load_failed() {
        // hw_verified error model unchanged: a `PRG0:` resource naming an absent file → Load
        // failed (46), even when slot 0 is the (non-running) target.
        let err = run_slot1_with_txt("EXEC 1", "EXEC \"PRG0:NOPE\"\nEND", &[])
            .err()
            .expect("missing file");
        assert_eq!(err.errnum(), Some(46));
    }

    #[test]
    fn use_string_into_non_running_slot_0_marks_common_def_callable() {
        // The slot-0 USE edge (M6-T6): `USE "PRG0:file"` for a *non-running* slot 0 loads the
        // program and marks slot 0 executable, uniform with the other slots — its COMMON DEF
        // then resolves cross-slot via `CALL "name"`. Boot slot 0 EXECs slot 1; from slot 1
        // (slot 0 now non-running) `USE "PRG0:LIB"` loads LIB into slot 0 and `CALL "SHOUT"`
        // runs its COMMON DEF.
        let vm = run_slot1_with_txt(
            "EXEC 1",
            "USE \"PRG0:LIB\"\nCALL \"SHOUT\"\nSTOP",
            &[("LIB", "COMMON DEF SHOUT\nPRINT \"HEY\"\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "HEY");
    }

    #[test]
    fn use_string_loads_common_def_callable_cross_slot() {
        // documented (use.yaml + common.yaml): `USE "PRGn:file"` loads + marks the slot
        // executable, so a COMMON DEF in that file resolves via `CALL "name"` from slot 0.
        let vm = run_with_txt(
            "USE \"PRG1:LIB\"\nCALL \"SHOUT\"",
            &[("LIB", "COMMON DEF SHOUT\nPRINT \"HEY\"\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "HEY");
    }

    #[test]
    fn use_string_loaded_common_def_passes_args_and_out() {
        // The loaded COMMON DEF carries full DEF call semantics: value args bind, OUT results
        // return into the caller's slot-0 variable.
        let vm = run_with_txt(
            "USE \"PRG3:LIB\"\nCALL \"ADD\",4,5 OUT R\nPRINT R",
            &[("LIB", "COMMON DEF ADD A,B OUT C\nC=A+B\nEND")],
        )
        .expect("run");
        assert_eq!(vm.console_text(), "9");
    }

    #[test]
    fn use_string_missing_file_is_load_failed() {
        // hw_verified (use.yaml): a valid non-running slot whose file is absent → Load failed (46).
        let err = run_with_txt("USE \"PRG1:NOPE\"", &[])
            .err()
            .expect("missing file");
        assert_eq!(err.errnum(), Some(46));
    }

    #[test]
    fn use_string_bare_name_defaults_to_running_slot_errnum_4() {
        // hw_verified (use.yaml, sb-oracle 2026-06-23): a bare filename string with no `PRGn:`
        // resource number defaults to the RUNNING slot (osb `Exec.execute` rule), which you
        // cannot USE — so it always raises Illegal function call (4). The running-slot guard
        // precedes the file-existence check, so a *missing* bare-name file still → 4, not the
        // 46 a missing `PRGn:` file gives (`USE "NOPE"`→4, `USE "Q"`→4).
        assert_eq!(
            run_with_txt("USE \"NOPE\"", &[])
                .err()
                .and_then(|e| e.errnum()),
            Some(4)
        );
        assert_eq!(
            run_with_txt("USE \"Q\"", &[])
                .err()
                .and_then(|e| e.errnum()),
            Some(4)
        );
        // Even when the bare-name file EXISTS, the running-slot guard still fires (the file
        // check never runs) → 4, proving the bare name resolved to the running slot.
        assert_eq!(
            run_with_txt("USE \"P\"", &[("P", "DEF F\nEND")])
                .err()
                .and_then(|e| e.errnum()),
            Some(4)
        );
    }

    #[test]
    fn unbounded_recursion_is_stack_overflow() {
        // A DEF that always recurses must trip Stack overflow (errnum 5).
        let err = run_err("DEF LOOP\nLOOP\nEND\nLOOP");
        assert_eq!(err.errnum(), Some(5));
    }

    #[test]
    fn def_recursion_trip_depth_matches_real_sb() {
        // hw_verified 2026-06-26 (sb-oracle binary search,
        // harness/harvest/out/stackdepth.tsv): real SB 3.6.0 completes a DEF recursion of
        // depth 2730 and trips Stack overflow (errnum 5) at depth 2731. `DEF F:N=N+1:IF N<D
        // THEN F` recurses while N<D, so D frames are pushed before the recursion stops;
        // D=2730 must run clean (run() does not panic), D=2731 must raise errnum 5.
        run("N=0\nDEF F\nN=N+1\nIF N<2730 THEN F\nEND\nF");
        let err = run_err("N=0\nDEF F\nN=N+1\nIF N<2731 THEN F\nEND\nF");
        assert_eq!(err.errnum(), Some(5), "depth 2731 must trip Stack overflow");
    }

    #[test]
    fn gosub_nesting_trip_depth_matches_real_sb() {
        // hw_verified 2026-06-26 (sb-oracle binary search,
        // harness/harvest/out/stackdepth_gosub.tsv): real SB 3.6.0 completes a GOSUB
        // nesting of depth 16384 and trips Stack overflow (errnum 5) at depth 16385. The
        // GOSUB return-address stack is separate from the DEF call-frame stack (~6×
        // deeper). D=16384 must run clean, D=16385 must raise errnum 5.
        run("N=0\nGOSUB @A\nEND\n@A\nN=N+1\nIF N<16384 THEN GOSUB @A\nRETURN");
        let err = run_err("N=0\nGOSUB @A\nEND\n@A\nN=N+1\nIF N<16385 THEN GOSUB @A\nRETURN");
        assert_eq!(
            err.errnum(),
            Some(5),
            "depth 16385 must trip Stack overflow"
        );
    }

    #[test]
    fn errline_points_at_failing_line() {
        // The divide-by-zero is on line 2.
        match run_err("A=1\nB=A/0") {
            VmError::Sb { errnum, line } => {
                assert_eq!(errnum, 7);
                assert_eq!(line, 2);
            }
            other => panic!("expected Sb error, got {other:?}"),
        }
    }

    // ---- error model: ERRNUM / ERRLINE / ERRPRG sysvars (M1-T13) ----

    /// Compile + run, returning the VM even when the run halts with an error (so the
    /// post-halt error-state residue can be inspected). Uses the builtin registry.
    fn run_to_halt(src: &str) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        let _ = vm.run();
        vm
    }

    #[test]
    fn errnum_for_documented_error_cases() {
        // hw_verified codes (errors.yaml / error-model.md): type mismatch, divide by
        // zero, subscript out of range, illegal function call, out of range.
        assert_eq!(run_err("A=FLOOR(\"x\")").errnum(), Some(8));
        assert_eq!(run_err("A=1/0").errnum(), Some(7));
        assert_eq!(run_err("DIM A[3]\nA[5]=1").errnum(), Some(31));
        assert_eq!(run_err("A=ABS()").errnum(), Some(4));
        assert_eq!(run_err("A=SQR(-1)").errnum(), Some(10));
    }

    #[test]
    fn error_state_persists_after_halt() {
        // After a halting error, ERRNUM/ERRLINE/ERRPRG are readable (the DIRECT-mode
        // residue). The SQR(-1) is on line 2; single-slot → ERRPRG = 0.
        let vm = run_to_halt("A=0\nB=SQR(-1)");
        assert_eq!(vm.errnum(), 10);
        assert_eq!(vm.errline(), 2);
        assert_eq!(vm.errprg(), 0);
    }

    #[test]
    fn errnum_reads_zero_on_a_clean_run() {
        // No error yet ⇒ ERRNUM/ERRLINE/ERRPRG read 0 mid-program (errnum 0 = No Error).
        let vm = run("N=ERRNUM\nL=ERRLINE\nP=ERRPRG");
        assert_eq!(int(&vm, "N"), 0);
        assert_eq!(int(&vm, "L"), 0);
        assert_eq!(int(&vm, "P"), 0);
        // A clean END leaves ERRNUM = 0.
        assert_eq!(vm.errnum(), 0);
    }

    #[test]
    fn error_sysvars_are_read_only() {
        // Assigning to a read-only error sysvar is a Syntax error (errnum 3) at compile.
        for src in ["ERRNUM=5", "ERRLINE=1", "ERRPRG=2"] {
            let ast = parse(src).expect("parse");
            let err = crate::compiler::compile(&ast).expect_err("expected a compile error");
            assert_eq!(err.errnum, 3, "{src} should be Syntax error");
        }
    }

    // ---- builtin calls (M1-T7) ----

    #[test]
    fn math_builtins_via_assignment() {
        // Paren calls compile to CallBuiltin even without the registry.
        let vm = run("A=FLOOR(3.7)\nB=ABS(-5)\nC=MAX(3,2.5)\nD=MIN(1,2,3,4)");
        assert_eq!(real(&vm, "A"), 3.0);
        assert_eq!(int(&vm, "B"), 5);
        assert_eq!(real(&vm, "C"), 3.0); // 2 mixed args -> Double (hw_verified)
        assert_eq!(real(&vm, "D"), 1.0); // 3+ args -> always Double (hw_verified)
    }

    #[test]
    fn niladic_pi_call() {
        // PI() is a 0-arg paren call -> CallBuiltin, Double pi.
        let vm = run("A=PI()");
        assert!((real(&vm, "A") - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn bare_pi_resolves_with_registry() {
        // With the builtin registry, the bare name `PI` (no parens) is a call, not a var.
        let vm = run_b("A=PI*2");
        assert!((real(&vm, "A") - std::f64::consts::TAU).abs() < 1e-12);
    }

    #[test]
    fn nested_builtin_calls() {
        // SQR(POW(3,2)) = SQR(9) = 3.
        let vm = run("A=SQR(POW(3,2))");
        assert_eq!(real(&vm, "A"), 3.0);
    }

    #[test]
    fn string_builtins_via_assignment() {
        let vm = run(r#"A$=LEFT$("ABCDEF",3)
B$=MID$("ABCDEF",2,2)
C=LEN("HELLO")
D=INSTR("ABCDEF","CD")
E$=CHR$(65)
F=ASC("Z")
G$=STR$(123)
H$=HEX$(255)"#);
        assert_eq!(string(&vm, "A"), "ABC");
        assert_eq!(string(&vm, "B"), "CD");
        assert_eq!(int(&vm, "C"), 5);
        assert_eq!(int(&vm, "D"), 2);
        assert_eq!(string(&vm, "E"), "A");
        assert_eq!(int(&vm, "F"), 90);
        assert_eq!(string(&vm, "G"), "123");
        assert_eq!(string(&vm, "H"), "FF");
    }

    #[test]
    fn builtin_value_arg_errors_propagate() {
        // SQR(-1) -> Out of range (10); FLOOR("x") -> Type mismatch (8).
        assert_eq!(run_err("A=SQR(-1)").errnum(), Some(10));
        assert_eq!(run_err(r#"A=FLOOR("x")"#).errnum(), Some(8));
        assert_eq!(run_err(r#"A=LEFT$("ABC",-1)"#).errnum(), Some(10));
    }

    #[test]
    fn function_as_statement_raises_illegal_function_call() {
        // A value-returning builtin in the whole-paren statement form is NOT silently
        // discarded — real SB 3.6.0 raises Illegal function call (4) (bead
        // sb-interpreter-5iu, hw_verified harness/harvest/out/exprstmt2.tsv: FLOOR(2.5),
        // ABS(5), SQR(4)… all → errnum 4). The split is per-builtin: GCLS()/RGB()/PI() →
        // Syntax error 3 instead (rejected at parse). See [`crate::parser::expr_stmt_class`].
        assert_eq!(run_err("FLOOR(3.7)").errnum(), Some(4));
        assert_eq!(run_err("ABS(5)").errnum(), Some(4));
        assert_eq!(run_err("SQR(4)").errnum(), Some(4));

        // errnum 4 is deferred to RUNTIME (the handler raises it), so a preceding statement
        // on the same line still runs: the PRINT output survives before the error.
        let ast = parse(r#"PRINT "HI":ABS(5)"#).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        let err = vm.run().expect_err("expected illegal function call");
        assert_eq!(err.errnum(), Some(4));
        assert!(vm.console_text().contains("HI"));
    }

    // ---- RNG (M1-T9): RND / RNDF / RANDOMIZE through the full program path ----

    #[test]
    fn seeded_rnd_sequence_matches_otya_golden() {
        // The `otya_test.sb3` fixture (real-SB golden): after `RANDOMIZE 0,1`, the first
        // four `RND(100)` draws are 89,33,33,52; `RNDF` ≈ 0.836095; next `RND(100)` == 66.
        let vm = run(
            "RANDOMIZE 0,1\nA=RND(100)\nB=RND(100)\nC=RND(100)\nD=RND(100)\nF=RNDF(0)\nG=RND(100)",
        );
        assert_eq!(int(&vm, "A"), 89);
        assert_eq!(int(&vm, "B"), 33);
        assert_eq!(int(&vm, "C"), 33);
        assert_eq!(int(&vm, "D"), 52);
        assert_eq!(format!("{:.6}", real(&vm, "F")), "0.836095");
        assert_eq!(int(&vm, "G"), 66);
    }

    #[test]
    fn rnd_two_arg_selects_series() {
        // The two-arg form picks the series; same seed → same golden as series 0.
        let vm = run("RANDOMIZE 5,1\nA=RND(5,100)");
        assert_eq!(int(&vm, "A"), 89);
    }

    #[test]
    fn rnd_one_is_zero() {
        let vm = run("RANDOMIZE 0,1\nA=RND(1)");
        assert_eq!(int(&vm, "A"), 0);
    }

    #[test]
    fn rng_error_conditions() {
        // RND(-1): max < 0 → Out of range (10).
        assert_eq!(run_err("A=RND(-1)").errnum(), Some(10));
        // RND(8,5): seed_id 8 out of 0-7 → Out of range (10).
        assert_eq!(run_err("A=RND(8,5)").errnum(), Some(10));
        // RND("x"): string arg → Type mismatch (8).
        assert_eq!(run_err(r#"A=RND("x")"#).errnum(), Some(8));
        // RNDF(8): seed_id out of range → 10; RNDF("x") → 8.
        assert_eq!(run_err("A=RNDF(8)").errnum(), Some(10));
        assert_eq!(run_err(r#"A=RNDF("x")"#).errnum(), Some(8));
        // RANDOMIZE 8 → 10; RANDOMIZE "x" → 8.
        assert_eq!(run_err("RANDOMIZE 8").errnum(), Some(10));
        assert_eq!(run_err(r#"RANDOMIZE "x""#).errnum(), Some(8));
    }

    // ---- console output: PRINT (M1-T8) ----

    /// Run a program and return its console text (the deterministic `stdout`).
    fn out(src: &str) -> String {
        run(src).console_text()
    }

    #[test]
    fn print_integer_and_string() {
        assert_eq!(out("PRINT 42"), "42");
        assert_eq!(out(r#"PRINT "HI""#), "HI");
        assert_eq!(out("PRINT 2*3+1"), "7");
    }

    #[test]
    fn named_constants_resolve_to_their_values() {
        // `#NAME` constants fold to inline Integer literals (hw_verified S-T14c).
        assert_eq!(out("PRINT #UP"), "1");
        assert_eq!(out("PRINT #L"), "256");
        assert_eq!(out("PRINT #R"), "512");
        // A color word is the signed i32 of its ARGB value (&HFFF8F8F8 -> -460552).
        assert_eq!(out("PRINT #WHITE"), "-460552");
        assert_eq!(out("PRINT #RED"), "-524288");
        // In an expression and as a DATA item.
        assert_eq!(out("PRINT #L+1"), "257");
        assert_eq!(out("READ A\nDATA #L\nPRINT A"), "256");
    }

    #[test]
    fn print_negative_number_has_no_leading_space() {
        // SmileBASIC does NOT pad positive numbers with a leading space (unlike MS BASIC).
        assert_eq!(out("PRINT -5"), "-5");
        assert_eq!(out("PRINT 7"), "7");
    }

    #[test]
    fn print_real_uses_g_format() {
        assert_eq!(out("PRINT 7/2"), "3.5");
        assert_eq!(out("PRINT 1.0"), "1");
    }

    #[test]
    fn print_semicolon_concatenates_with_no_gap() {
        assert_eq!(out(r#"PRINT "A";"B""#), "AB");
        assert_eq!(out("PRINT 1;2;3"), "123");
    }

    #[test]
    fn print_comma_tabs_to_next_tabstep() {
        // TABSTEP default 4: "1" at col 0, tab to col 4, "2".
        assert_eq!(out("PRINT 1,2"), "1   2");
    }

    #[test]
    fn print_comma_from_tab_boundary_advances_full_step() {
        // hw_verified (M7-T2 run 5): a cursor already sitting on a TABSTEP boundary still
        // advances a full step. "ABCD" fills cols 0-3, the comma from col 4 jumps to col 8.
        assert_eq!(out(r#"PRINT "ABCD","Z""#), "ABCD    Z");
    }

    #[test]
    fn print_semicolon_number_then_string_no_gap() {
        // hw_verified (M7-T2 run 5): integer %d, no trailing space, `;` no-gap -> "5X".
        assert_eq!(out(r#"PRINT 5;"X""#), "5X");
        assert_eq!(out("PRINT -7"), "-7");
    }

    #[test]
    fn print_trailing_comma_suppresses_newline() {
        // hw_verified (M7-T2 run 5): a trailing `,` suppresses the closing newline, so the
        // next PRINT shares the row; "X" at col 0, tab to col 4, "Y".
        assert_eq!(out(r#"PRINT "X",:PRINT "Y""#), "X   Y");
    }

    #[test]
    fn print_bare_emits_one_blank_line() {
        // hw_verified (M7-T2 run 5): a bare PRINT emits exactly one line break.
        assert_eq!(out("PRINT \"A\"\nPRINT\nPRINT \"B\""), "A\n\nB");
    }

    #[test]
    fn print_question_alias() {
        assert_eq!(out(r#"?"Q""#), "Q");
    }

    #[test]
    fn print_multiple_lines() {
        assert_eq!(out(r#"PRINT "A":PRINT "B""#), "A\nB");
    }

    #[test]
    fn print_trailing_semicolon_suppresses_newline() {
        // Two PRINTs, the first `;`-terminated, share a line.
        assert_eq!(out(r#"PRINT "A";:PRINT "B""#), "AB");
    }

    #[test]
    fn print_type_mismatch_is_errnum_8() {
        // A bare string/number mix that produces a non-printable value can't arise from a
        // literal here; PRINT of an array name is a Type mismatch.
        assert_eq!(run_err("DIM A[3]\nPRINT A").errnum(), Some(8));
    }

    // ---- console state: LOCATE / COLOR / CLS (M1-T8) ----

    #[test]
    fn locate_then_print_positions_text() {
        // `;` suppresses the trailing newline so the cursor stays where the text left it.
        let vm = run(r#"LOCATE 5,2:PRINT "X";"#);
        assert_eq!(vm.console().cell(5, 2).ch, u16::from(b'X'));
        // After printing one char the cursor advanced to col 6.
        assert_eq!((vm.console().cur_x, vm.console().cur_y), (6, 2));
    }

    #[test]
    fn color_sets_cell_palette() {
        let vm = run(r#"COLOR 3,4:PRINT "X""#);
        assert_eq!(vm.console().cell(0, 0).fg, 3);
        assert_eq!(vm.console().cell(0, 0).bg, 4);
    }

    #[test]
    fn cls_clears_console() {
        // hw_verified: PRINT then CLS leaves the console empty.
        // The bare `CLS` command needs the registry so it is not mistaken for a bare
        // variable used as a statement (which is Syntax error 3).
        assert_eq!(run_b(r#"PRINT "X":CLS"#).console_text(), "");
    }

    #[test]
    fn console_command_error_conditions() {
        // hw_verified expects from the console specs. Uses the registry helper so the
        // bare `BACKCOLOR` is recognized as a command (→ its arg-count errnum 4), not a
        // bare-variable statement (→ Syntax error 3).
        assert_eq!(run_b_err("LOCATE 51,0").errnum(), Some(10)); // X out of range
        assert_eq!(run_b_err("LOCATE 0,30").errnum(), Some(10)); // Y out of range
        assert_eq!(run_b_err("LOCATE 0,0,2000").errnum(), Some(10)); // Z out of range
        assert_eq!(run_b_err("LOCATE 0").errnum(), Some(4)); // single slot
        assert_eq!(run_b_err("COLOR 16").errnum(), Some(10)); // fg out of range
        assert_eq!(run_b_err("COLOR 0,16").errnum(), Some(10)); // bg out of range
        assert_eq!(run_b_err("CLS 0").errnum(), Some(4)); // CLS takes no args
        assert_eq!(run_b_err("BACKCOLOR").errnum(), Some(4)); // SET needs 1 arg
        assert_eq!(run_b_err("BACKCOLOR 0,1").errnum(), Some(4)); // too many
        assert_eq!(run_b_err("ACLS 1").errnum(), Some(4)); // 1 arg illegal
        assert_eq!(run_b_err("ACLS 1,1").errnum(), Some(4)); // 2 args illegal
    }

    #[test]
    fn console_commands_as_functions_error() {
        assert_eq!(run_err("A=LOCATE(0,0)").errnum(), Some(4));
        assert_eq!(run_err("A=COLOR(7)").errnum(), Some(4));
        assert_eq!(run_err("A=CLS()").errnum(), Some(4));
    }

    #[test]
    fn x_edge_50_wraps_not_panics() {
        // LOCATE 50 (off-screen edge) is legal and must not panic; printing wraps to the
        // next row, so "X" still lands on the console.
        assert!(out(r#"LOCATE 50,0:PRINT "X""#).contains('X'));
    }

    #[test]
    fn acls_runs_and_resets() {
        // The no-arg form and the corpus-verified 3-arg form both run; no error.
        run_b(r#"COLOR 3,4:PRINT "X":ACLS"#);
        run_b("ACLS 1,1,0");
    }

    #[test]
    fn acls_resets_draw_settings_but_not_tabstep() {
        // hw_verified (acls.yaml, M7-T2 2026-06-24): ACLS restores the scalar draw settings
        // to their power-on defaults (GCOLOR -> -1 white, BACKCOLOR -> 0, WIDTH -> 8,
        // DISPLAY -> 0) but leaves the writable TABSTEP sysvar untouched.
        let vm = run_b(
            "GCOLOR RGB(1,2,3):BACKCOLOR &HFF112233:WIDTH 16:XSCREEN 2:DISPLAY 1:TABSTEP=8:ACLS\n\
             G=GCOLOR():B=BACKCOLOR():W=WIDTH():D=DISPLAY():T=TABSTEP",
        );
        assert_eq!(int(&vm, "G"), -1);
        assert_eq!(int(&vm, "B"), 0);
        assert_eq!(int(&vm, "W"), 8);
        assert_eq!(int(&vm, "D"), 0);
        assert_eq!(int(&vm, "T"), 8);
        // The 3-arg selective form resets identically.
        let vm = run_b("BACKCOLOR &HFF112233:WIDTH 16:ACLS 1,1,1\nB=BACKCOLOR():W=WIDTH()");
        assert_eq!(int(&vm, "B"), 0);
        assert_eq!(int(&vm, "W"), 8);
    }

    #[test]
    fn backcolor_round_trips() {
        // SET then GET returns the stored color code (low 24 bits — fits, unchanged).
        let vm = run("BACKCOLOR 12345\nA=BACKCOLOR()");
        assert_eq!(int(&vm, "A"), 12345);
    }

    #[test]
    fn backcolor_get_strips_alpha() {
        // The hardware keeps only a 24-bit RGB border color: the GET round-trip drops the
        // high (alpha) byte (sb-oracle batch 2026-06-23, `backcolor.yaml` hw_verified).
        // RGB(64,128,128) = &HFF408080 -> &H408080 = 4227200.
        assert_eq!(
            int(&run("BACKCOLOR RGB(64,128,128)\nA=BACKCOLOR()"), "A"),
            4227200
        );
        // RGB(16,32,64) = &HFF102040 -> &H102040 = 1056832 (clean strip, no channel swap).
        assert_eq!(
            int(&run("BACKCOLOR RGB(16,32,64)\nA=BACKCOLOR()"), "A"),
            1056832
        );
        // A non-0xFF high byte is dropped just the same: &H7F112233 -> &H112233.
        assert_eq!(
            int(&run("BACKCOLOR &H7F112233\nA=BACKCOLOR()"), "A"),
            0x11_2233
        );
        // -1 = &HFFFFFFFF -> &HFFFFFF = 16777215.
        assert_eq!(int(&run("BACKCOLOR -1\nA=BACKCOLOR()"), "A"), 16_777_215);
    }

    // ---- Screen fader: FADE/FADECHK (M4/M5 frame effect) ----

    #[test]
    fn fade_error_conditions() {
        // Wrong call shapes raise errnum 4. Registry helper so bare `FADE` is the command
        // (→ errnum 4 for the missing arg), not a bare-variable statement (→ errnum 3).
        assert_eq!(run_b_err("FADE").errnum(), Some(4)); // statement needs >=1 arg
        assert_eq!(run_b_err("FADE 0,0,0").errnum(), Some(4)); // too many
        assert_eq!(run_b_err("A=FADE(0)").errnum(), Some(4)); // function takes no args
        assert_eq!(run_b_err("FADECHK 0").errnum(), Some(4)); // statement form forbidden
        // `FADECHK()` is the EMPTY-paren statement form (0 args). Per the corrected arg-count
        // model (bead sb-interpreter-imk; hw_verified `exprstmt_argcount.tsv` / committed in
        // `cases/exprstmt_split.yaml` as `GCOLOR()`/`ABS()`/`SPPAGE()`→3), empty parens are a
        // parse-time Syntax error (3) for EVERY builtin — not the runtime Illegal-fn-call (4)
        // the pre-correction reading assumed. The parser now consults the whole builtin
        // registry, so FADECHK (a value getter) is rejected at parse like every other builtin.
        assert_eq!(parse("FADECHK()").expect_err("syntax error").errnum, 3);
        // Negative fade time raises errnum 10 (Out of range).
        assert_eq!(run_b_err("FADE RGB(0,0,0,255),-1").errnum(), Some(10));
    }

    #[test]
    fn fade_round_trips_argb() {
        // FADE() returns the full ARGB code; alpha byte is preserved, unlike BACKCOLOR.
        // RGB uses (A,R,G,B) order in SmileBASIC.
        let vm = run("FADE RGB(1,2,3,4)\nA=FADE()");
        assert_eq!(int(&vm, "A"), 0x0102_0304u32 as i32);
        let vm = run("FADE RGB(128,64,128,128)\nA=FADE()");
        assert_eq!(int(&vm, "A"), 0x8040_8080u32 as i32);
    }

    #[test]
    fn fade_zero_disables() {
        // Setting a fully-transparent color disables the fader; FADE() still reads it back as 0.
        let vm = run("FADE RGB(255,255,0,0)\nFADE 0\nA=FADE()");
        assert_eq!(int(&vm, "A"), 0);
    }

    #[test]
    fn acls_resets_fader() {
        let vm = run_b("FADE RGB(128,255,0,0),10\nACLS\nA=FADE()");
        assert_eq!(int(&vm, "A"), 0);
    }

    #[test]
    fn fadechk_reports_animation_state() {
        // 1-arg FADE is instant: FADECHK() stays FALSE (0).
        assert_eq!(out("FADE RGB(255,0,0,0)\nPRINT FADECHK()"), "0");
        // Timed fade is still animating when checked immediately: TRUE (1).
        assert_eq!(out("FADE RGB(255,0,0,0),10\nPRINT FADECHK()"), "1");
        // After WAIT elapses the animation is complete: FALSE (0).
        assert_eq!(out("FADE RGB(255,0,0,0),10\nWAIT 10\nPRINT FADECHK()"), "0");
    }

    #[test]
    fn fade_animation_advances_per_tick() {
        // Headless WAIT drives the fader in a burst; after WAIT 5 of a 10-frame fade the
        // remaining time is 5, so FADECHK() is still TRUE.
        assert_eq!(out("FADE RGB(255,0,0,0),10\nWAIT 5\nPRINT FADECHK()"), "1");
        // And after the full duration it is FALSE.
        assert_eq!(out("FADE RGB(255,0,0,0),10\nWAIT 10\nPRINT FADECHK()"), "0");
    }

    // ---- Screen configuration: XSCREEN/DISPLAY/VISIBLE/HARDWARE (M4-T4) ----

    #[test]
    fn xscreen_runs_documented_forms() {
        // The 1-arg and 3-arg forms both run without error (corpus-ubiquitous shapes).
        run("XSCREEN 0");
        run("XSCREEN 3,512,4");
        run("XSCREEN 2,128,4"); // the doc example
    }

    #[test]
    fn xscreen_tracks_per_screen_allocations() {
        // XSCREEN records the per-screen sprite/BG split in ScreenConfig (#83/#84 — tracked,
        // not enforced). Mode-only forms use the per-mode defaults; the 3-arg form assigns the
        // `sprites`/`bg` counts to the UPPER screen and the remainder to the Touch screen.
        // Modes 2/3 (Touch Screen) default to a 256/256 sprite + 2/2 BG split.
        let vm = run("XSCREEN 2");
        assert_eq!(vm.screen_config().sprite_alloc(), [256, 256]);
        assert_eq!(vm.screen_config().bg_alloc(), [2, 2]);
        // Mode 0 (Upper only): all 512 sprites / 4 BG to the Upper screen, none to the Touch.
        let vm = run("XSCREEN 0");
        assert_eq!(vm.screen_config().sprite_alloc(), [512, 0]);
        assert_eq!(vm.screen_config().bg_alloc(), [4, 0]);
        // Explicit split: the doc example assigns 128 sprites / 4 BG to the Upper screen.
        let vm = run("XSCREEN 2,128,4");
        assert_eq!(vm.screen_config().sprite_alloc(), [128, 384]);
        assert_eq!(vm.screen_config().bg_alloc(), [4, 0]);
    }

    #[test]
    fn xscreen_error_conditions() {
        // hw_verified (sb-oracle batch s_t11d, xscreen.yaml).
        assert_eq!(run_err("XSCREEN 2,128").errnum(), Some(4)); // 2 args illegal
        assert_eq!(run_err("XSCREEN 5").errnum(), Some(10)); // mode out of range
        assert_eq!(run_err("XSCREEN 2,513,4").errnum(), Some(10)); // sprites > 512
        assert_eq!(run_err("XSCREEN 2,256,5").errnum(), Some(10)); // BG > 4
        assert_eq!(run_b_err("A=XSCREEN(2)").errnum(), Some(4)); // no return value
    }

    #[test]
    fn display_get_round_trips() {
        // The GET form reports the currently selected screen; default is the Upper screen 0.
        let vm = run_b("DISPLAY 0:A=DISPLAY()");
        assert_eq!(int(&vm, "A"), 0);
        // XSCREEN 2 exposes the Touch Screen, so DISPLAY 1 is accepted and read back.
        let vm = run_b("XSCREEN 2:DISPLAY 1:A=DISPLAY()");
        assert_eq!(int(&vm, "A"), 1);
    }

    #[test]
    fn display_error_conditions() {
        // hw_verified (sb-oracle batch s_t11d, display.yaml).
        assert_eq!(run_err("DISPLAY 0,1").errnum(), Some(4)); // SET needs exactly 1 arg
        assert_eq!(run_err("XSCREEN 0:DISPLAY 1").errnum(), Some(10)); // Touch unavailable
        assert_eq!(run_b_err("A=DISPLAY(0)").errnum(), Some(4)); // GET takes no args
    }

    #[test]
    fn display_routes_grp_commands_to_the_selected_screen() {
        // The Touch screen defaults to displaying GRP1 (osb showPage = [0, 1]); the active
        // (command-target) screen starts as the Upper screen.
        let vm = run("XSCREEN 2");
        assert_eq!(vm.grp().screens[1].display_page, 1);
        assert_eq!(vm.grp().active, 0);

        // DISPLAY 1 (under XSCREEN 2) selects the Touch screen, so the following GPAGE/GPRIO
        // mutate screen 1's draw context, leaving screen 0 (Upper) untouched.
        let vm = run("XSCREEN 2:DISPLAY 1:GPAGE 4,5:GPRIO 100");
        assert_eq!(vm.grp().active, 1);
        assert_eq!(vm.grp().screens[1].display_page, 4);
        assert_eq!(vm.grp().screens[1].manip_page, 5);
        assert_eq!(vm.grp().screens[1].prio, 100);
        // Screen 0 keeps its defaults — the per-screen split isolates the contexts.
        assert_eq!(vm.grp().screens[0].display_page, 0);
        assert_eq!(vm.grp().screens[0].manip_page, 0);
        assert_eq!(vm.grp().screens[0].prio, 0);

        // Returning to DISPLAY 0 re-targets the Upper screen for subsequent commands.
        let vm = run("XSCREEN 2:DISPLAY 1:GPAGE 4,5:DISPLAY 0:GPAGE 3,2");
        assert_eq!(vm.grp().active, 0);
        assert_eq!(vm.grp().screens[0].display_page, 3);
        assert_eq!(vm.grp().screens[0].manip_page, 2);
        assert_eq!(vm.grp().screens[1].display_page, 4); // screen 1 unchanged
        assert_eq!(vm.grp().screens[1].manip_page, 5);
    }

    #[test]
    fn console_state_is_per_display_screen() {
        // Each physical screen owns a console sized to its resolution.
        let vm = run_b("XSCREEN 2");
        assert_eq!((vm.console_for(0).cols, vm.console_for(0).rows), (50, 30));
        assert_eq!((vm.console_for(1).cols, vm.console_for(1).rows), (40, 30));

        // PRINT and COLOR route to whichever DISPLAY is selected.
        let vm = run_b("XSCREEN 2:DISPLAY 1:COLOR 3,4:PRINT \"B\":DISPLAY 0:PRINT \"A\"");
        assert_eq!(vm.console_for(1).cell(0, 0).ch, u16::from(b'B'));
        assert_eq!(vm.console_for(1).cell(0, 0).fg, 3);
        assert_eq!(vm.console_for(0).cell(0, 0).ch, u16::from(b'A'));
        assert_eq!(vm.console_for(0).cell(0, 0).fg, 15); // default white

        // CLS clears only the active screen's console.
        let vm = run_b("XSCREEN 2:DISPLAY 1:PRINT \"B\":DISPLAY 0:PRINT \"A\":CLS");
        assert_eq!(vm.console_for(0).cell(0, 0).ch, 0);
        assert_eq!(vm.console_for(1).cell(0, 0).ch, u16::from(b'B'));
    }

    #[test]
    fn console_sysvars_and_chkchr_follow_active_screen() {
        // CSRX/CSRY read the active screen's cursor; CHKCHR reads the active screen's grid.
        let vm = run_b(
            "XSCREEN 2:DISPLAY 1:LOCATE 10,5:A=CSRX:B=CSRY:PRINT \"B\":C=CHKCHR(10,5)\n\
             DISPLAY 0:LOCATE 20,7:D=CSRX:E=CSRY:PRINT \"A\":F=CHKCHR(20,7)",
        );
        assert_eq!(int(&vm, "A"), 10);
        assert_eq!(int(&vm, "B"), 5);
        assert_eq!(int(&vm, "C"), u16::from(b'B') as i32);
        assert_eq!(int(&vm, "D"), 20);
        assert_eq!(int(&vm, "E"), 7);
        assert_eq!(int(&vm, "F"), u16::from(b'A') as i32);
    }

    #[test]
    fn locate_range_matches_active_console_dimensions() {
        // The Touch screen is 40 columns wide, so LOCATE 40,0 is the off-screen edge and
        // LOCATE 41,0 is out of range. The Upper screen stays 50 columns wide.
        run_b("XSCREEN 2:DISPLAY 1:LOCATE 40,0");
        assert_eq!(
            run_b_err("XSCREEN 2:DISPLAY 1:LOCATE 41,0").errnum(),
            Some(10)
        );
        run_b("XSCREEN 2:DISPLAY 0:LOCATE 50,0");
        assert_eq!(
            run_b_err("XSCREEN 2:DISPLAY 0:LOCATE 51,0").errnum(),
            Some(10)
        );
    }

    #[test]
    fn xscreen_resizes_the_active_console_to_match_resolution() {
        // Mode 1 makes the Upper screen 320×240, so the top console becomes 40×30.
        let vm = run_b("XSCREEN 1");
        assert_eq!((vm.console_for(0).cols, vm.console_for(0).rows), (40, 30));
    }

    #[test]
    fn acls_resets_consoles_to_default_dimensions() {
        // After ACLS the screen mode returns to 0, so the top console is restored to 50×30
        // and its grid is cleared.
        let vm = run_b("XSCREEN 1:PRINT \"X\":ACLS");
        assert_eq!((vm.console_for(0).cols, vm.console_for(0).rows), (50, 30));
        assert_eq!(vm.console_for(0).cell(0, 0).ch, 0);
    }

    #[test]
    fn acls_resets_grp_command_target_to_upper() {
        // ACLS resets DISPLAY -> 0 (hw_verified, acls.yaml). The GRP command-target screen
        // must follow, or subsequent draws would route to the Touch screen while DISPLAY()
        // reads back 0.
        let vm = run_b("XSCREEN 2:DISPLAY 1:ACLS");
        assert_eq!(vm.grp().active, 0);
        assert_eq!(vm.screen_config().display, 0);
    }

    #[test]
    fn sprites_are_per_display_screen() {
        // A sprite created under DISPLAY 1 lives in screen 1's table and is absent from
        // screen 0's, and vice-versa (#83). XSCREEN 2 exposes the Touch screen.
        let vm = run("XSCREEN 2:DISPLAY 1:SPSET 0,0:DISPLAY 0:SPSET 5,0");
        // Screen 1 (Touch) has sprite 0 but not 5; screen 0 (Upper) has sprite 5 but not 0.
        assert!(vm.sprites_for(1).is_used(0));
        assert!(!vm.sprites_for(1).is_used(5));
        assert!(vm.sprites_for(0).is_used(5));
        assert!(!vm.sprites_for(0).is_used(0));

        // SPUSED() queries the ACTIVE screen, so it reflects whichever DISPLAY is selected.
        let vm = run("XSCREEN 2:DISPLAY 1:SPSET 0,0:DISPLAY 0:A=SPUSED(0):B=SPUSED(5)\nSPSET 5,0:B=SPUSED(5)");
        assert_eq!(int(&vm, "A"), 0); // sprite 0 not on the Upper screen
        assert_eq!(int(&vm, "B"), 1); // sprite 5 created on the Upper screen
    }

    #[test]
    fn spdef_is_shared_across_screens() {
        // SPDEF is a GLOBAL definition table — an edit made under one DISPLAY is visible after
        // switching to the other screen (it is NOT duplicated per screen, #83). Define template
        // 10 under DISPLAY 0, switch to DISPLAY 1, SPSET from template 10, and confirm the
        // sprite copied the shared template's rectangle.
        let vm = run("XSCREEN 2:SPDEF 10,40,48,24,32,7,9,1:DISPLAY 1:SPSET 0,10");
        let sp = &vm.sprites_for(1).sprites[0];
        assert_eq!((sp.u, sp.v, sp.w, sp.h), (40, 48, 24, 32));
        // The shared table itself reads back the edit regardless of the active screen.
        let t = vm.spdef().get(10);
        assert_eq!((t.u, t.v, t.w, t.h), (40, 48, 24, 32));
    }

    #[test]
    fn visible_runs_and_gates_layers() {
        // hw_verified arg guard (visible.yaml): exactly 4 arguments.
        assert_eq!(run_err("VISIBLE 1,1,1").errnum(), Some(4)); // too few
        assert_eq!(run_err("VISIBLE 1,1,1,1,1").errnum(), Some(4)); // too many
        assert_eq!(run_b_err("A=VISIBLE(1,1,1,1)").errnum(), Some(4)); // no return value
                                                                       // The four flags booleanize onto the selected screen's layer visibility.
        let vm = run("VISIBLE 0,1,0,1");
        let v = vm.screen_visibility();
        assert!(!v.console && v.graphic && !v.bg && v.sprite);
        // Any nonzero shows the layer; all-ON restores the full stack.
        let vm = run("VISIBLE 2,2,2,2");
        let v = vm.screen_visibility();
        assert!(v.console && v.graphic && v.bg && v.sprite);
    }

    // Regression repro for chainlink #87: the RPG GAME textbox drawn at the bottom of the
    // Upper screen must keep its bottom border; a trailing PRINT newline on the last row
    // must not scroll the freshly-printed border off the screen.
    #[test]
    fn rpg_textbox_bottom_border_not_scrolled() {
        use sb_render::bg::BgState;
        use sb_render::compositor::{compose_top_screen, LayerVisibility, DEFAULT_BACKDROP};
        use sb_render::sprite::SpriteState;
        let src = "XSCREEN 2\n\
            GPRIO 1024\n\
            VISIBLE 1,1,1,1\n\
            TX=20:TY=23:TW=30:TH=7\n\
            COLOR 3\n\
            X=TX:Y=TY:Z=-100:W=TW:H=TH\n\
            GOSUB @TEXTBOX\n\
            GLINE 0,232,399,232,RGB(255,255,255)\n\
            END\n\
            @TEXTBOX\n\
            LOCATE X,Y,Z\n\
            PRINT \"\";\n\
            PRINT \"\"*(W-2);\n\
            PRINT \"\"\n\
            FOR I=1 TO H-2\n\
             LOCATE X,Y+I,Z:PRINT \"\";\n\
             PRINT \" \"*(W-2);\n\
             PRINT \"\"\n\
            NEXT\n\
            LOCATE X,Y+H-1,Z:PRINT \"\";\n\
            PRINT \"\"*(W-2);\n\
            PRINT \"\"\n\
            RETURN";
        let vm = run_b(src);
        let fb = compose_top_screen(
            vm.grp(),
            &BgState::new(),
            &SpriteState::new(),
            vm.console_for(0),
            DEFAULT_BACKDROP,
            LayerVisibility::default(),
        );
        // If the trailing newline scrolled the bottom row away, the cells would be empty.
        let console = vm.console_for(0);
        let bottom_left_ch = console.cell(20, 29).ch;
        let bottom_right_ch = console.cell(20 + 29, 29).ch;
        let inside_bottom_ch = console.cell(20 + 15, 29).ch;
        assert_ne!(
            bottom_left_ch, 0,
            "bottom-left cell should contain a border char"
        );
        assert_ne!(
            bottom_right_ch, 0,
            "bottom-right cell should contain a border char"
        );
        assert_ne!(
            inside_bottom_ch, 0,
            "bottom border cell should contain a border char"
        );
        // Sample pixels on the actual glyph strokes (the box glyphs are 2px-thick lines).
        let bottom_left = fb.get_argb(20 * 8 + 4, 29 * 8 + 4);
        let bottom_right = fb.get_argb((20 + 29) * 8 + 3, 29 * 8 + 3);
        let inside_bottom = fb.get_argb((20 + 15) * 8 + 4, 29 * 8 + 4);
        // The corners/border are red; the GRP line behind is white.
        assert_eq!(bottom_left, 0xFFF8_0000, "bottom-left corner should be red");
        assert_eq!(
            bottom_right, 0xFFF8_0000,
            "bottom-right corner should be red"
        );
        assert_eq!(inside_bottom, 0xFFF8_0000, "bottom border should be red");
    }

    #[test]
    fn hardware_reports_the_model() {
        // HARDWARE reads as a bare-name sysvar (1 = new3DS, the Azahar/oracle value).
        let vm = run_b("H=HARDWARE");
        assert_eq!(int(&vm, "H"), 1);
    }

    #[test]
    fn hardware_is_read_only() {
        // Assigning to the read-only sysvar is a compile-time Syntax error (errnum 3).
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("HARDWARE=2").expect("parse");
        let err = compile_with(&ast, &StdBuiltins).expect_err("assignment must be rejected");
        assert_eq!(err.errnum, 3);
    }

    // ---- INKEY$ (M1-T8) ----

    #[test]
    fn inkey_is_empty_headless() {
        // No live keyboard buffer headless → "".
        assert_eq!(out("C$=INKEY$():PRINT LEN(C$)"), "0");
        assert_eq!(run_err("C$=INKEY$(1)").errnum(), Some(4));
    }

    // ---- CHKCHR (M1-T14) ----

    #[test]
    fn chkchr_reads_console_grid() {
        // Round-trip: print a glyph, read its UTF-16 code back, then CLS so the scrape is the
        // read-back value alone (ASC("A") == 65).
        assert_eq!(
            run_b(r#"LOCATE 0,0:?"A";:C=CHKCHR(0,0):CLS:?C"#).console_text(),
            "65"
        );
        // Empty / out-of-bounds cells read as 0 (no error).
        assert_eq!(out("?CHKCHR(10,10)"), "0");
        assert_eq!(out("?CHKCHR(-1,0)"), "0");
        assert_eq!(out("?CHKCHR(0,100)"), "0");
        // Wrong arg count (function) and statement use both → errnum 4.
        assert_eq!(run_err("C=CHKCHR(0)").errnum(), Some(4));
        assert_eq!(run_err("CHKCHR 0,0").errnum(), Some(4));
    }

    // ---- INPUT / LINPUT (M1-T8) ----

    /// Run with a preloaded input queue, returning the VM for inspection.
    fn run_with_input(src: &str, lines: &[&str]) -> Vm {
        let ast = parse(src).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);
        for l in lines {
            vm.push_input(l);
        }
        vm.run().expect("run");
        vm
    }

    #[test]
    fn input_numeric_and_string() {
        let vm = run_with_input("INPUT A\nINPUT B$", &["42", "hello"]);
        assert_eq!(int(&vm, "A"), 42);
        assert_eq!(string(&vm, "B"), "hello");
    }

    #[test]
    fn input_multiple_comma_fields() {
        let vm = run_with_input("INPUT A,B,C", &["1,2,3"]);
        assert_eq!(int(&vm, "A"), 1);
        assert_eq!(int(&vm, "B"), 2);
        assert_eq!(int(&vm, "C"), 3);
    }

    #[test]
    fn input_real_field() {
        let vm = run_with_input("INPUT R", &["3.5"]);
        assert_eq!(real(&vm, "R"), 3.5);
    }

    #[test]
    fn input_literal_receiver_is_syntax_error() {
        // `INPUT "X";1` — a literal receiver is rejected (errnum 3); the parser catches it
        // before compilation (hw_verified: real SB raises errnum 3 for a non-variable
        // receiver).
        let err = parse(r#"INPUT "X";1"#).expect_err("syntax error");
        assert_eq!(err.errnum, 3);
    }

    #[test]
    fn linput_keeps_commas() {
        let vm = run_with_input("LINPUT S$", &["a,b,c"]);
        assert_eq!(string(&vm, "S"), "a,b,c");
    }

    #[test]
    fn input_prompt_is_printed() {
        // The guide text + `?` show before the (queued) input is read.
        let vm = run_with_input(r#"INPUT "NAME";A$"#, &["bob"]);
        assert_eq!(string(&vm, "A"), "bob");
        assert!(vm.console_text().starts_with("NAME?"));
    }

    #[test]
    fn interactive_input_yields_until_enter() {
        let ast = parse(r#"INPUT "NAME";A$"#).expect("parse");
        let program = compile(&ast).expect("compile");
        let mut vm = Vm::new(program);

        // First frame prints the prompt and yields, waiting for interactive input.
        assert_eq!(vm.run_frame(1000).unwrap(), None);
        assert!(vm.awaiting_input());
        assert!(vm.console_text().starts_with("NAME?"));

        // Type the answer and submit it.
        for c in "bob".chars() {
            vm.input_char(c as u32);
        }
        vm.input_enter();
        assert!(!vm.awaiting_input());

        // Next frame consumes the line and the program ends.
        assert_eq!(vm.run_frame(1000).unwrap(), Some(Halt::End));
        assert_eq!(string(&vm, "A"), "bob");
    }

    // -- M3-T5 BG extras (VM orchestration) ------------------------------------

    #[test]
    fn bganim_inline_advances_scroll_on_vsync() {
        // Inline XY keyframe: hold (16,8). VSYNC advances the BG frame clock one step.
        let vm = run_b("BGSCREEN 0,32,32\nBGANIM 0,\"XY\",2,16,8\nVSYNC 1");
        assert_eq!((vm.bg().layers[0].ofs_x, vm.bg().layers[0].ofs_y), (16, 8));
    }

    #[test]
    fn bganim_array_form_drives_rotation() {
        // Numeric target 4 (R), array data [1,90] = one hold keyframe of 90 degrees.
        let vm = run_b("DIM A[2]\nA[0]=1\nA[1]=90\nBGANIM 0,4,A\nVSYNC 1");
        assert_eq!(vm.bg().layers[0].rot, 90);
    }

    #[test]
    fn bganim_data_label_form() {
        // "@label" form: first DATA value is the keyframe count, then Time,Item for Z.
        let vm = run_b("BGANIM 0,\"Z\",\"@AD\"\nVSYNC 1\nEND\n@AD\nDATA 1\nDATA 1,5");
        assert_eq!(vm.bg().layers[0].ofs_z, 5);
    }

    #[test]
    fn bganim_too_few_args_errors() {
        // Fewer than 3 arguments -> Illegal function call (4).
        assert!(matches!(
            run_b_err("BGANIM 0,\"XY\""),
            VmError::Sb { errnum: 4, .. }
        ));
        // Layer out of range -> 10.
        assert!(matches!(
            run_b_err("BGANIM 4,\"XY\",10,0,0"),
            VmError::Sb { errnum: 10, .. }
        ));
    }

    #[test]
    fn bgchk_reflects_running_animation() {
        // After BGANIM on Z (channel 1), BGCHK has bit 1 set (#CHKZ = 2).
        let vm = run_b("BGANIM 0,\"Z\",10,5\nST=BGCHK(0)\nPRINT ST");
        assert_eq!(vm.console_text(), "2");
        // After BGSTOP the layer reads 0.
        let vm = run_b("BGANIM 0,\"Z\",10,5\nBGSTOP 0\nPRINT BGCHK(0)");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn bgstart_bgstop_isolate_named_layer() {
        // Two layers animating XY (#CHKXY = 1). hw_verified M7-T2 run 35: the one-layer form
        // touches only the named layer; the no-arg form touches all.
        let prog = "BGANIM 0,0,60,100,100,0\nBGANIM 1,0,60,100,100,0";
        // Baseline: both running.
        let vm = run_b(&format!("{prog}\nPRINT BGCHK(0);BGCHK(1)"));
        assert_eq!(vm.console_text(), "11");
        // Stop layer 0 only — layer 1 stays running.
        let vm = run_b(&format!("{prog}\nBGSTOP 0\nPRINT BGCHK(0);BGCHK(1)"));
        assert_eq!(vm.console_text(), "01");
        // Stop all, resume layer 0 only.
        let vm = run_b(&format!(
            "{prog}\nBGSTOP\nBGSTART 0\nPRINT BGCHK(0);BGCHK(1)"
        ));
        assert_eq!(vm.console_text(), "10");
        // Resume all; idempotent double BGSTART is a no-op (no error).
        let vm = run_b(&format!(
            "{prog}\nBGSTOP\nBGSTART\nBGSTART\nPRINT BGCHK(0);BGCHK(1)"
        ));
        assert_eq!(vm.console_text(), "11");
    }

    #[test]
    fn bgfunc_binds_callback_name() {
        let vm = run_b("BGSCREEN 0,32,32\nBGFUNC 0,@CB\nEND\n@CB");
        assert_eq!(vm.bg().layers[0].func.as_deref(), Some("CB"));
    }

    #[test]
    fn bgfunc_errors() {
        // Unresolvable label -> Illegal function call (4).
        assert!(matches!(
            run_b_err("BGFUNC 0,\"@NOPE\""),
            VmError::Sb { errnum: 4, .. }
        ));
        // Layer out of range -> 10.
        assert!(matches!(
            run_b_err("BGFUNC 4,@P\nEND\n@P"),
            VmError::Sb { errnum: 10, .. }
        ));
        // Non-string label -> Type mismatch (8).
        assert!(matches!(
            run_b_err("BGFUNC 0,5"),
            VmError::Sb { errnum: 8, .. }
        ));
    }

    // ---- CALL SPRITE / CALL BG callback dispatch (M6-T6) ----

    #[test]
    fn call_sprite_runs_bound_label_callback_with_callidx() {
        // CALL SPRITE invokes sprite 0's SPFUNC-bound @label process; inside it CALLIDX is
        // the sprite's management number (documented). The @label runs GOSUB-style (RETURN).
        let vm = run_b("SPSET 0,0\nSPFUNC 0,@CB\nCALL SPRITE\nEND\n@CB\nPRINT CALLIDX\nRETURN");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn call_sprite_dispatches_all_bound_in_ascending_order() {
        // Every bound sprite process runs, in ascending management-number order, each with
        // its own CALLIDX. Sprites 0 and 3 are bound; the `;` keeps them on one line.
        let vm = run_b(
            "SPSET 0,0\nSPSET 3,0\nSPFUNC 0,@CB\nSPFUNC 3,@CB\nCALL SPRITE\nEND\n@CB\nPRINT CALLIDX;\nRETURN",
        );
        assert_eq!(vm.console_text(), "03");
    }

    #[test]
    fn call_sprite_runs_bound_def_process() {
        // A SPFUNC name that resolves to a DEF process (not an @label) is invoked like a
        // 0-arg CALL; CALLIDX is still the management number inside it.
        let vm = run_b("SPSET 2,0\nSPFUNC 2,@CB\nCALL SPRITE\nEND\nDEF CB\nPRINT CALLIDX\nEND");
        assert_eq!(vm.console_text(), "2");
    }

    #[test]
    fn call_sprite_with_no_bound_callbacks_is_noop() {
        // No SPFUNC bound anywhere: CALL SPRITE sweeps the table and falls through.
        let vm = run_b("SPSET 0,0\nCALL SPRITE\nPRINT \"OK\"");
        assert_eq!(vm.console_text(), "OK");
    }

    #[test]
    fn call_bg_runs_bound_layer_callback_with_callidx() {
        // CALL BG invokes each BGFUNC-bound layer's process; inside it CALLIDX is the layer.
        let vm = run_b("BGSCREEN 0,32,32\nBGFUNC 0,@CB\nCALL BG\nEND\n@CB\nPRINT CALLIDX\nRETURN");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn call_bg_dispatches_all_bound_layers_in_order() {
        let vm = run_b(
            "BGSCREEN 0,32,32\nBGFUNC 0,@CB\nBGFUNC 2,@CB\nCALL BG\nEND\n@CB\nPRINT CALLIDX;\nRETURN",
        );
        assert_eq!(vm.console_text(), "02");
    }

    #[test]
    fn call_bg_leaves_callidx_one_past_the_layer_table() {
        // osb VM.d: after CALL BG the shared CALLIDX counter rests at 4 (one past layers 0..3).
        let vm = run_b("BGSCREEN 0,32,32\nCALL BG\nPRINT CALLIDX");
        assert_eq!(vm.console_text(), "4");
    }

    #[test]
    fn callidx_reads_zero_before_any_callback_sweep() {
        // The idle/boot value is 0 (hw_verified sysvars golden); a sweep is what changes it.
        let vm = run_b("PRINT CALLIDX");
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn bgvar_round_trip_through_vm() {
        let vm = run_b("BGSCREEN 0,32,32\nBGVAR 0,3,7\nPRINT BGVAR(0,3)");
        assert_eq!(vm.console_text(), "7");
    }

    #[test]
    fn bgsave_load_round_trip_through_vm() {
        // Write a couple of cells, save the whole screen, reload into another layer.
        let vm = run_b(
            "BGSCREEN 0,8,8\nBGSCREEN 1,8,8\nBGPUT 0,1,1,&H1234\nDIM A[64]\nBGSAVE 0,A\nBGLOAD 1,A\nPRINT BGGET(1,1,1)",
        );
        assert_eq!(vm.console_text(), "4660"); // &H1234
    }

    #[test]
    fn bg_layers_are_per_display_screen() {
        // BG tilemaps are per-DISPLAY-screen (#84). A tile written under DISPLAY 1 lives in
        // screen 1's table and is absent from screen 0's, and vice-versa. XSCREEN 2 exposes the
        // Touch screen. BGSCREEN sizes the active screen's layer; BGPUT/BGGET hit that screen.
        let vm = run_b(
            "XSCREEN 2:DISPLAY 1:BGSCREEN 0,8,8:BGPUT 0,1,1,&H1234\nDISPLAY 0:BGSCREEN 0,8,8:BGPUT 0,2,2,&H5678",
        );
        // Cell index is row-major `y * width + x` (width 8): (1,1) → 9, (2,2) → 18.
        // Screen 1 (Touch) holds &H1234 at (1,1) and nothing at (2,2).
        assert_eq!(vm.bg_for(1).layers[0].cells[9], 0x1234);
        assert_eq!(vm.bg_for(1).layers[0].cells[18], 0);
        // Screen 0 (Upper) holds &H5678 at (2,2) and nothing at (1,1).
        assert_eq!(vm.bg_for(0).layers[0].cells[18], 0x5678);
        assert_eq!(vm.bg_for(0).layers[0].cells[9], 0);

        // BGGET() reads the ACTIVE screen, so it reflects whichever DISPLAY is selected.
        let vm = run_b(
            "XSCREEN 2:DISPLAY 1:BGSCREEN 0,8,8:BGPUT 0,1,1,&H1234\nDISPLAY 0:A=BGGET(0,1,1)\nDISPLAY 1:B=BGGET(0,1,1)",
        );
        assert_eq!(int(&vm, "A"), 0); // Upper screen layer is empty at (1,1)
        assert_eq!(int(&vm, "B"), 0x1234); // Touch screen has the tile

        // BGPAGE is per-screen (like SPPAGE), no shared sub-table — distinct values stick.
        let vm = run_b("XSCREEN 2:DISPLAY 1:BGPAGE 3\nDISPLAY 0:BGPAGE 5");
        assert_eq!(vm.bg_for(1).page, 3);
        assert_eq!(vm.bg_for(0).page, 5);
    }

    // ---- hardware input (BUTTON/STICK/STICKEX/BREPEAT, M4-T1) ----

    /// Compile (with the builtin registry), let `setup` fill the input snapshot, then run.
    fn run_b_input(src: &str, setup: impl FnOnce(&mut crate::input::InputState)) -> Vm {
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse(src).expect("parse");
        let program = compile_with(&ast, &StdBuiltins).expect("compile");
        let mut vm = Vm::new(program);
        setup(vm.input_mut());
        vm.run().expect("run");
        vm
    }

    #[test]
    fn button_reads_held_mask_through_vm() {
        // Hold A (16) + RIGHT (8); BUTTON() reports the combined mask.
        let vm = run_b_input("PRINT BUTTON()", |i| {
            i.advance_frame(16 | 8, (0.0, 0.0), (0.0, 0.0));
        });
        assert_eq!(vm.console_text(), "24");
    }

    #[test]
    fn button_feature_edges_through_vm() {
        // After a press-then-hold, feature 2 (raw pressed) clears but feature 0 (held) stays.
        let vm = run_b_input("PRINT BUTTON(0);\",\";BUTTON(2)", |i| {
            i.advance_frame(16, (0.0, 0.0), (0.0, 0.0)); // press A
            i.advance_frame(16, (0.0, 0.0), (0.0, 0.0)); // hold A
        });
        assert_eq!(vm.console_text(), "16,0");
    }

    #[test]
    fn button_statement_use_is_errnum_4() {
        // BUTTON requires exactly one result; as a bare statement it raises errnum 4.
        assert!(matches!(run_b_err("BUTTON"), VmError::Sb { errnum: 4, .. }));
    }

    #[test]
    fn stick_writes_axes_through_vm() {
        let vm = run_b_input("STICK OUT X,Y\nPRINT X;\",\";Y", |i| {
            i.advance_frame(0, (0.5, -0.25), (0.0, 0.0));
        });
        assert_eq!(vm.console_text(), "0.5,-0.25");
    }

    #[test]
    fn stickex_reads_right_stick_through_vm() {
        let vm = run_b_input("STICKEX OUT X,Y\nPRINT X;\",\";Y", |i| {
            i.advance_frame(0, (0.0, 0.0), (-1.0, 1.0));
        });
        assert_eq!(vm.console_text(), "-1,1");
    }

    #[test]
    fn brepeat_then_button_feature1_refires() {
        // BREPEAT runs in-program (configuring repeat); a later same-VM frame timeline drives
        // the re-fire. Here we just confirm BREPEAT commits without error and BUTTON reads.
        let vm = run_b_input("BREPEAT 4,1,2\nPRINT BUTTON(1)", |i| {
            i.advance_frame(16, (0.0, 0.0), (0.0, 0.0)); // press A -> feature 1 fires
        });
        assert_eq!(vm.console_text(), "16");
    }

    #[test]
    fn brepeat_reserved_id_is_errnum_4() {
        assert!(matches!(
            run_b_err("BREPEAT 10,15,4"),
            VmError::Sb { errnum: 4, .. }
        ));
        assert!(matches!(
            run_b_err("BREPEAT 13,15,4"),
            VmError::Sb { errnum: 4, .. }
        ));
    }

    #[test]
    fn button_wireless_terminal_is_comms_error() {
        // The 2-arg terminal form hits the wireless path; with no multiplayer it raises 52.
        assert!(matches!(
            run_b_err("A=BUTTON(0,1)"),
            VmError::Sb { errnum: 52, .. }
        ));
    }

    // ---- touch + keyboard (TOUCH/KEY/INKEY$, M4-T2) ----

    #[test]
    fn touch_writes_three_out_vars_through_vm() {
        let vm = run_b_input("TOUCH OUT TM,TX,TY\nPRINT TM;\",\";TX;\",\";TY", |i| {
            i.advance_touch(true, 100, 50); // STTM 1, coords 100,50
        });
        assert_eq!(vm.console_text(), "1,100,50");
    }

    #[test]
    fn touch_no_touch_baseline_is_zero() {
        // Headless / no touch: STTM reads 0 (the documented no-touch value).
        let vm = run_b_input("TOUCH OUT TM,TX,TY\nPRINT TM", |_| {});
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn touch_empty_out_slots_discard_results() {
        // `TOUCH OUT TM,,` keeps only the touch time; the two omitted slots still count.
        let vm = run_b_input("TOUCH OUT TM,,\nPRINT TM", |i| {
            i.advance_touch(true, 7, 9);
        });
        assert_eq!(vm.console_text(), "1");
        // `TOUCH OUT ,TX,TY` discards the time, keeps the coordinates.
        let vm = run_b_input("TOUCH OUT ,TX,TY\nPRINT TX;\",\";TY", |i| {
            i.advance_touch(true, 7, 9);
        });
        assert_eq!(vm.console_text(), "7,9");
    }

    #[test]
    fn touch_wrong_out_count_is_errnum_4() {
        assert!(matches!(
            run_b_err("TOUCH OUT TM,TX"),
            VmError::Sb { errnum: 4, .. }
        ));
    }

    #[test]
    fn touch_wireless_terminal_is_comms_error() {
        assert!(matches!(
            run_b_err("TOUCH 1 OUT TM,TX,TY"),
            VmError::Sb { errnum: 52, .. }
        ));
    }

    #[test]
    fn key_assign_and_function_read_round_trip() {
        // KEY 1,"HI" binds slot 1; the undocumented KEY(1) function form reads it back.
        let vm = run_b_input("KEY 1,\"HI\"\nPRINT KEY(1)", |_| {});
        assert_eq!(vm.console_text(), "HI");
    }

    #[test]
    fn key_unset_slot_reads_empty() {
        let vm = run_b_input("PRINT LEN(KEY(5))", |_| {});
        assert_eq!(vm.console_text(), "0");
    }

    #[test]
    fn key_number_out_of_range_is_errnum_10() {
        assert!(matches!(
            run_b_err("KEY 6,\"X\""),
            VmError::Sb { errnum: 10, .. }
        ));
        assert!(matches!(
            run_b_err("KEY 0,\"X\""),
            VmError::Sb { errnum: 10, .. }
        ));
    }

    #[test]
    fn key_nonstring_value_is_errnum_8() {
        assert!(matches!(
            run_b_err("KEY 3,5"),
            VmError::Sb { errnum: 8, .. }
        ));
    }

    #[test]
    fn inkey_drains_queued_keys_through_vm() {
        // INKEY$ pops one queued code unit per call; the empty queue yields "".
        let vm = run_b_input("PRINT INKEY$();INKEY$();LEN(INKEY$())", |i| {
            i.push_key(b'A' as u16);
            i.push_key(b'B' as u16);
        });
        assert_eq!(vm.console_text(), "AB0");
    }

    // -- M4-T3 frame timing: MAINCNT + VSYNC/WAIT over the frame clock ------------

    #[test]
    fn maincnt_starts_at_zero() {
        // A fresh program has never advanced a frame, so MAINCNT reads 0.
        assert_eq!(run_b("?MAINCNT").console_text(), "0");
    }

    #[test]
    fn wait_advances_maincnt_by_the_count() {
        // WAIT counts from the present frame: WAIT 60 leaves MAINCNT at 60.
        assert_eq!(run_b("WAIT 60\n?MAINCNT").console_text(), "60");
        // A bare WAIT defaults to one frame.
        assert_eq!(run_b("WAIT\n?MAINCNT").console_text(), "1");
        // WAIT 0 ("0: Ignore") does not advance.
        assert_eq!(run_b("WAIT 0\n?MAINCNT").console_text(), "0");
    }

    #[test]
    fn vsync_loop_advances_maincnt_one_per_frame() {
        // Five `VSYNC 1`s, each anchored at the previous VSYNC, advance MAINCNT to 5.
        assert_eq!(
            run_b("FOR I=0 TO 4\nVSYNC 1\nNEXT\n?MAINCNT").console_text(),
            "5"
        );
    }

    #[test]
    fn maincnt_difference_measures_elapsed_frames() {
        // The idiom `MAINCNT - start`: capture, wait, and the delta is the frames blocked.
        assert_eq!(run_b("S=MAINCNT\nWAIT 30\n?MAINCNT-S").console_text(), "30");
    }

    #[test]
    fn tick_frame_advances_maincnt_and_animation() {
        // The platform heartbeat: set up a BG scroll animation, return from the program,
        // then drive frames from the host loop — MAINCNT advances and the scroll steps.
        let mut vm = run_b("BGSCREEN 0,32,32\nBGANIM 0,\"XY\",2,16,8");
        assert_eq!(vm.maincnt(), 0);
        vm.tick_frame();
        assert_eq!(vm.maincnt(), 1);
        assert_eq!((vm.bg().layers[0].ofs_x, vm.bg().layers[0].ofs_y), (16, 8));
    }

    #[test]
    fn tick_frames_advances_multiple_frames_at_once() {
        // Wall-clock catch-up: a host that knows several 1/60s frames have elapsed can
        // advance the clock/animations by the full count in one call, matching the result
        // of stepping one frame at a time.
        let mut fast = run_b("BGSCREEN 0,32,32\nBGANIM 0,\"XY\",2,16,8");
        fast.tick_frames(5);
        let mut slow = run_b("BGSCREEN 0,32,32\nBGANIM 0,\"XY\",2,16,8");
        for _ in 0..5 {
            slow.tick_frame();
        }
        assert_eq!(fast.maincnt(), 5);
        assert_eq!(fast.maincnt(), slow.maincnt());
        assert_eq!(
            (fast.bg().layers[0].ofs_x, fast.bg().layers[0].ofs_y),
            (slow.bg().layers[0].ofs_x, slow.bg().layers[0].ofs_y)
        );
    }

    #[test]
    fn maincnt_is_read_only() {
        // MAINCNT is writable=false: assigning is a compile-time Syntax error (errnum 3).
        use crate::builtins::StdBuiltins;
        use crate::compiler::compile_with;
        let ast = parse("MAINCNT=5").expect("parse");
        let err = compile_with(&ast, &StdBuiltins).expect_err("MAINCNT is read-only");
        assert_eq!(err.errnum, 3);
    }

    // -- interactive (run_frame + tick_frame) timing tests (#94) -------------------
    //
    // These exercise the host-driven VBlank model: run_frame arms a pending wait via
    // begin_wait/begin_vsync, then tick_frame drives the counter one frame at a time.
    // The program only resumes once tick_frame has advanced the clock to the target.

    /// Run a program to completion under the interactive model, driving it with run_frame
    /// + tick_frame up to `max_frames` host ticks. Returns the VM once it halts (or panics
    ///   if it hasn't halted within `max_frames`).
    fn run_interactive(src: &str, max_frames: usize) -> Vm {
        let prog = compile(&parse(src).expect("parse")).expect("compile");
        let mut vm = Vm::new(prog);
        let budget = 100_000;
        for _ in 0..max_frames {
            vm.tick_frame();
            match vm.run_frame(budget) {
                Ok(Some(_)) => return vm,
                Ok(None) => {}
                Err(e) => panic!("runtime error: {:?}", e),
            }
        }
        panic!("program did not halt within {} frames", max_frames);
    }

    #[test]
    fn run_frame_wait_blocks_one_frame_at_a_time() {
        // WAIT 3 must block for exactly 3 host ticks, not jump in one shot.
        // The first tick_frame fires before execution, so frame=1 when WAIT runs and
        // arms target=4; three more ticks drive it to 4. MAINCNT == 4.
        let vm = run_interactive("WAIT 3\n?MAINCNT", 10);
        assert_eq!(vm.console_text(), "4");
    }

    #[test]
    fn run_frame_vsync_1_loop_advances_maincnt_one_per_tick() {
        // Five VSYNC 1 iterations (I=0..4): the initial tick puts frame=1 before the
        // first run_frame, so each VSYNC 1 sees frame already at last_vsync+1 (immediate
        // resync) and still yields, adding one tick each. MAINCNT == 5+1 == 6.
        let vm = run_interactive("FOR I=0 TO 4\nVSYNC 1\nNEXT\n?MAINCNT", 20);
        assert_eq!(vm.console_text(), "6");
    }

    #[test]
    fn run_frame_maincnt_advances_without_vsync() {
        // A program that does no VSYNC/WAIT still sees MAINCNT advance because tick_frame
        // drives the counter before every run_frame call.  After 5 ticks a program that
        // loops printing MAINCNT once per iteration (using a counter) sees values 1..=5.
        // We verify this by checking MAINCNT > 0 at halt after the host drives 3 ticks
        // before the program even starts executing.
        let prog = compile(&parse("?MAINCNT").expect("parse")).expect("compile");
        let mut vm = Vm::new(prog);
        // Drive 3 ticks *before* any run_frame so MAINCNT starts at 3.
        vm.tick_frame();
        vm.tick_frame();
        vm.tick_frame();
        match vm.run_frame(100_000) {
            Ok(Some(_)) => {}
            _ => panic!("should halt immediately"),
        }
        assert_eq!(vm.console_text(), "3");
    }

    #[test]
    fn run_frame_wait_zero_does_not_yield() {
        // WAIT 0 ("0: Ignore") must not block — the program advances past it immediately
        // in the same run_frame call.
        let vm = run_interactive("WAIT 0\n?MAINCNT", 5);
        // MAINCNT == 1 because tick_frame fires once before run_frame.
        assert_eq!(vm.console_text(), "1");
    }

    #[test]
    fn run_frame_vsync_loop_holds_steady_rate() {
        // Ten VSYNC 1 iterations: each blocks for exactly one host tick, so MAINCNT
        // advances by 10 over 10 ticks (plus the initial tick before execution).
        let vm = run_interactive("FOR I=1 TO 10\nVSYNC 1\nNEXT\n?MAINCNT", 25);
        // MAINCNT == 10+1=11 because tick_frame fires once before the first run_frame.
        assert_eq!(vm.console_text(), "11");
    }

    // ---- System variables (M6-T3) ----

    #[test]
    fn version_encodes_3_6_0() {
        // VERSION = &H03060000 = 50724864 (hw_verified golden, sysvars.yaml).
        assert_eq!(int(&run("V=VERSION"), "V"), 50_724_864);
        assert_eq!(out("PRINT VERSION"), "50724864");
    }

    #[test]
    fn date_and_time_strings_are_deterministic_under_the_injected_clock() {
        // The default headless epoch is 2000/01/01 00:00:00.
        assert_eq!(out("PRINT DATE$"), "2000/01/01");
        assert_eq!(out("PRINT TIME$"), "00:00:00");
        // Injecting a wall clock changes both, with zero-padded fields.
        let prog = compile(&parse("D$=DATE$\nT$=TIME$").expect("parse")).expect("compile");
        let mut vm = Vm::new(prog);
        vm.set_wall_clock(WallClock {
            year: 2026,
            month: 6,
            day: 9,
            hour: 7,
            minute: 4,
            second: 30,
        });
        vm.run().expect("run");
        assert_eq!(string(&vm, "D"), "2026/06/09");
        assert_eq!(string(&vm, "T"), "07:04:30");
    }

    #[test]
    fn date_and_time_need_the_dollar_suffix() {
        // TIME / DATE (no `$`) are ordinary numeric variables, not the string sysvars.
        let vm = run("TIME=5\nDATE=7");
        assert_eq!(int(&vm, "TIME"), 5);
        assert_eq!(int(&vm, "DATE"), 7);
    }

    #[test]
    fn tabstep_is_writable_and_takes_effect() {
        // Boot default 4; `TABSTEP=n` writes the VM state, and a read returns it.
        assert_eq!(out("PRINT TABSTEP"), "4");
        let vm = run("TABSTEP=8\nS=TABSTEP");
        assert_eq!(int(&vm, "S"), 8);
        // And the new width drives the `PRINT ,` tab: "1" at col 0, tab to col 8, "2".
        assert_eq!(out("TABSTEP=8\nPRINT \"1\",\"2\""), "1       2");
    }

    #[test]
    fn sysbeep_is_writable_and_round_trips() {
        // Boot default 1 (TRUE = beep allowed). `SYSBEEP=0` disables it; the flag round-trips
        // and is exposed to the platform UI via `Vm::sysbeep`.
        assert_eq!(out("PRINT SYSBEEP"), "1");
        let vm = run("SYSBEEP=0\nS=SYSBEEP");
        assert_eq!(int(&vm, "S"), 0);
        assert_eq!(vm.sysbeep(), 0);
        assert_eq!(run("SYSBEEP=1").sysbeep(), 1);
    }

    #[test]
    fn writable_sysvars_reject_a_string() {
        // TABSTEP/SYSBEEP are Integer: assigning a String is a Type mismatch (errnum 8).
        assert_eq!(run_err(r#"TABSTEP="x""#).errnum(), Some(8));
        assert_eq!(run_err(r#"SYSBEEP="x""#).errnum(), Some(8));
    }

    #[test]
    fn csrx_csry_track_the_text_cursor() {
        // After LOCATE the cursor sysvars report the column/row; CSRZ is a flat-grid 0.
        let vm = run("LOCATE 12,7\nX=CSRX\nY=CSRY\nZ=CSRZ");
        assert_eq!(int(&vm, "X"), 12);
        assert_eq!(int(&vm, "Y"), 7);
        assert_eq!(int(&vm, "Z"), 0);
    }

    #[test]
    fn freemem_and_stub_sysvars_read_their_model_values() {
        // hw_verified offline values (sb-oracle 2026-06-23): FREEMEM is a large positive
        // constant (8314876, near-empty-program snapshot); RESULT boots TRUE (1); CALLIDX/
        // MICPOS = 0; no session → MPCOUNT 0 but MPHOST/MPLOCAL = -1; PRGSLOT = running slot.
        let vm = run("F=FREEMEM\nR=RESULT\nC=CALLIDX\nP=PRGSLOT\nMC=MPCOUNT\nMH=MPHOST\nML=MPLOCAL\nMI=MICPOS");
        assert_eq!(int(&vm, "F"), 8_314_876);
        assert_eq!(int(&vm, "R"), 1);
        assert_eq!(int(&vm, "C"), 0);
        assert_eq!(int(&vm, "P"), 0);
        assert_eq!(int(&vm, "MC"), 0);
        assert_eq!(int(&vm, "MH"), -1);
        assert_eq!(int(&vm, "ML"), -1);
        assert_eq!(int(&vm, "MI"), 0);
    }

    #[test]
    fn read_only_sysvars_reject_assignment() {
        // Every non-writable system variable is a compile-time Syntax error (errnum 3) on write.
        for src in [
            "VERSION=1",
            "FREEMEM=1",
            "CSRX=1",
            "CSRY=1",
            "CSRZ=1",
            "RESULT=1",
            "PRGSLOT=1",
            "CALLIDX=1",
            "MPCOUNT=1",
            "MICPOS=1",
            "TIME$=\"x\"",
            "DATE$=\"x\"",
        ] {
            let ast = parse(src).expect("parse");
            let err = crate::compiler::compile(&ast).expect_err("read-only");
            assert_eq!(err.errnum, 3, "{src} should be a Syntax error");
        }
    }

    // ---- BGM commands: BGMPLAY/BGMSTOP/BGMCHK/BGMVAR/BGMVOL/BGMSET/BGMSETD/BGMCLEAR (M5-T3) ----

    #[test]
    fn bgmchk_tracks_play_and_stop() {
        // Fresh: nothing playing. BGMPLAY a tune, BGMCHK reports playing; BGMSTOP clears it.
        let vm = run_b("A=BGMCHK(0):BGMPLAY 0,27:B=BGMCHK(0):BGMSTOP 0:C=BGMCHK(0)");
        assert_eq!(int(&vm, "A"), 0);
        assert_eq!(int(&vm, "B"), 1);
        assert_eq!(int(&vm, "C"), 0);
    }

    #[test]
    fn bgmvar_round_trips_while_playing() {
        // Stopped: read returns 0 (hw_verified 2026-06-24 — docs' "-1 when stopped" is wrong
        // for 3.6.0). Playing: read returns the written value.
        let vm = run_b("BGMVAR 0,5,42:A=BGMVAR(0,5):BGMPLAY 0:B=BGMVAR(0,5)");
        assert_eq!(int(&vm, "A"), 0);
        assert_eq!(int(&vm, "B"), 42);
    }

    #[test]
    fn bgmset_then_play_user_tune() {
        // Compile an inline MML tune, register it under 128, then play it — no error.
        run_b(r#"BGMSET 128,"T120O4L4CDE":BGMPLAY 128"#);
    }

    #[test]
    fn bgmsetd_gathers_mml_from_data() {
        // The DATA-stored MML compiles + registers, then plays (the conformance gate's `basic`
        // case is excluded because it has no DATA block; this exercises the real happy path).
        run_b("BGMSETD 128,\"@MMLTOP\":BGMPLAY 128\n@MMLTOP\nDATA \"T120O4\",\"CDEFG\"\nDATA 0");
    }

    #[test]
    fn bgmsetd_missing_label_is_noerr() {
        // hw_verified (real SB 3.6.0, M7-T2 harness/harvest/out/bgmsetd.tsv): a label with no
        // matching DATA block is NOT an error — the RESTORE-shared lookup yields no MML, so an
        // empty tune is registered and the statement returns NOERR (not Undefined label / 14).
        run_b("BGMSETD 128,\"@NOPE\"");
    }

    #[test]
    fn bgm_malformed_mml_is_errnum_47() {
        // BGMSET / BGMPLAY of a string both surface the MML parser's Illegal MML (47).
        assert_eq!(run_b_err(r#"BGMSET 128,"+R""#).errnum(), Some(47));
        assert_eq!(run_b_err(r#"BGMPLAY "+R""#).errnum(), Some(47));
    }

    // ---- SFX / voice: BEEP/TALK/TALKCHK/TALKSTOP/EFC*/WAVSET/WAVSETA (M5-T4) ----

    #[test]
    fn beep_runs_and_skips_args() {
        // The bare form, a full form, and the empty-comma skip all run without error.
        run_b("BEEP:BEEP 20:BEEP 9,,80:BEEP 9,0,80,64");
    }

    #[test]
    fn beep_sound_gap_is_out_of_range() {
        // The 134..223 gap and >383 are Out of range (10); a function context is errnum 4.
        assert_eq!(run_b_err("BEEP 134").errnum(), Some(10));
        assert_eq!(run_b_err("BEEP 0,0,0,0,0").errnum(), Some(4));
    }

    #[test]
    fn talk_then_talkchk_then_stop() {
        // Idle TALKCHK is 0; after TALK it reports playing; TALKSTOP clears it.
        let vm = run_b(r#"A=TALKCHK():TALK "HELLO":B=TALKCHK():TALKSTOP:C=TALKCHK()"#);
        assert_eq!(int(&vm, "A"), 0);
        assert_eq!(int(&vm, "B"), 1);
        assert_eq!(int(&vm, "C"), 0);
    }

    #[test]
    fn talk_in_value_context_is_errnum_4() {
        assert_eq!(run_b_err(r#"X=TALK("HI")"#).errnum(), Some(4));
    }

    #[test]
    fn effector_set_on_off_wet() {
        // EFCSET preset + EFCON/EFCOFF + EFCWET all run; a bad arg count is Syntax error (3),
        // an out-of-range wet value is Out of range (10).
        run_b("EFCSET 2:EFCON:EFCWET 0,100,64:EFCOFF");
        assert_eq!(run_b_err("EFCSET 4").errnum(), Some(10));
        assert_eq!(run_b_err("EFCON 1").errnum(), Some(3));
        assert_eq!(run_b_err("EFCWET 0,0").errnum(), Some(3));
        assert_eq!(run_b_err("EFCWET 0,0,200").errnum(), Some(10));
    }

    #[test]
    fn wavset_defines_user_instrument() {
        // The hex-string form registers a user instrument (@224); the array form (WAVSETA)
        // reads a numeric sample array. A defnum outside 224..255 is Out of range (10).
        run_b(r#"WAVSET 224,3,10,30,5,"7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF""#);
        run_b("DIM SMP[16]:WAVSETA 255,120,0,127,124,SMP,45,0,15");
        assert_eq!(
            run_b_err(r#"WAVSET 223,3,10,30,5,"7F7F7F7FFFFFFFFF7F7F7F7FFFFFFFFF""#).errnum(),
            Some(10)
        );
        // A non-array WAVSETA source is Type mismatch (8).
        assert_eq!(run_b_err("WAVSETA 224,0,95,100,20,12345").errnum(), Some(8));
    }

    // ---- Source-code manipulation: PRG* family (M6-T4) ----

    /// Read a slot's lines as Strings (test helper over the private slot model).
    fn slot_lines(vm: &Vm, slot: usize) -> Vec<String> {
        vm.prg_slots[slot]
            .lines
            .iter()
            .map(|l| String::from_utf16_lossy(l))
            .collect()
    }

    #[test]
    fn prg_round_trip_edits_a_slot() {
        // Edit slot 1 (not the running slot 0): insert before, insert after, replace, read.
        let vm = run_b(
            r#"PRGEDIT 1
PRGINS "PRINT 1"
PRGINS "PRINT 2",1
PRGEDIT 1,0
PRGSET "REM HEAD"
A$=PRGGET$()
PRINT A$
PRINT PRGSIZE(1)"#,
        );
        assert_eq!(slot_lines(&vm, 1), ["REM HEAD", "PRINT 2"]);
        // PRGGET$ returns the current (first) line; PRGSIZE(1) is the 2-line count.
        assert_eq!(vm.console_text(), "REM HEAD\n2");
    }

    #[test]
    fn prg_del_removes_lines() {
        // Build three lines (insert-before at line 0 prepends), then PRGDEL the middle one.
        let vm = run_b(
            r#"PRGEDIT 2
PRGINS "C"
PRGINS "B"
PRGINS "A"
PRGEDIT 2,1
PRGDEL"#,
        );
        assert_eq!(slot_lines(&vm, 2), ["A", "C"]); // deleted the middle line "B"
        let vm2 = run_b(
            r#"PRGEDIT 2
PRGINS "B"
PRGINS "A"
PRGEDIT 2,0
PRGDEL -1"#,
        );
        assert!(slot_lines(&vm2, 2).is_empty()); // negative count deletes all remaining
    }

    #[test]
    fn prg_multiline_insert_splits_on_lf() {
        // A CHR$(10) in the inserted string adds multiple lines.
        let vm = run_b("PRGEDIT 3\nPRGINS \"X\"+CHR$(10)+\"Y\"");
        assert_eq!(slot_lines(&vm, 3), ["X", "Y"]);
    }

    #[test]
    fn prg_seeded_slot_reads_size_and_name() {
        // A host-seeded slot is readable by PRGSIZE / PRGNAME$ without PRGEDIT.
        let ast = parse("PRINT PRGSIZE(1);PRGNAME$(1)").expect("parse");
        let program = {
            use crate::compiler::compile_with;
            compile_with(&ast, &crate::builtins::StdBuiltins).expect("compile")
        };
        let mut vm = Vm::new(program);
        vm.set_slot_source(1, "MYPRG", "PRINT 1\nPRINT 2\nEND");
        vm.run().expect("run");
        assert_eq!(vm.console_text(), "3MYPRG"); // 3 lines, file name MYPRG
    }

    #[test]
    fn prg_cold_state_needs_prgedit() {
        // The four current-line mutators raise errnum 38 from a cold edit state.
        assert_eq!(run_b_err("A$=PRGGET$()").errnum(), Some(38));
        assert_eq!(run_b_err(r#"PRGSET "X""#).errnum(), Some(38));
        assert_eq!(run_b_err(r#"PRGINS "X""#).errnum(), Some(38));
        assert_eq!(run_b_err("PRGDEL").errnum(), Some(38));
    }

    #[test]
    fn prg_no_prgedit_check_precedes_arg_check() {
        // Cold state → 38 even with a bad arg (the 38 guard is checked first).
        assert_eq!(run_b_err("A$=PRGGET$(0)").errnum(), Some(38));
        // Warm (after PRGEDIT) → the arg-count guard is reached (errnum 4).
        assert_eq!(run_b_err("PRGEDIT 1:A$=PRGGET$(0)").errnum(), Some(4));
    }

    #[test]
    fn prgedit_guards() {
        assert_eq!(run_b_err("PRGEDIT 4").errnum(), Some(10)); // slot out of range
        assert_eq!(run_b_err("PRGEDIT -1").errnum(), Some(10)); // negative slot
        assert_eq!(run_b_err("PRGEDIT 0,0,0").errnum(), Some(4)); // too many args
        assert_eq!(run_b_err("PRGEDIT 0").errnum(), Some(4)); // running slot (0)
    }

    #[test]
    fn prg_mutator_arg_and_range_guards() {
        // Edit target active (slot 1), then trip each guard.
        assert_eq!(run_b_err(r#"PRGEDIT 1:PRGSET "A","B""#).errnum(), Some(4));
        assert_eq!(run_b_err(r#"PRGEDIT 1:PRGINS "A",1,2"#).errnum(), Some(4));
        assert_eq!(run_b_err("PRGEDIT 1:PRGDEL 1,2").errnum(), Some(4));
        assert_eq!(run_b_err("PRGEDIT 1:PRGDEL 0").errnum(), Some(10)); // count 0
    }

    #[test]
    fn prgname_and_prgsize_guards() {
        assert_eq!(run_b_err("A$=PRGNAME$(4)").errnum(), Some(10));
        assert_eq!(run_b_err("A$=PRGNAME$(-1)").errnum(), Some(10));
        assert_eq!(run_b_err("A$=PRGNAME$(0,1)").errnum(), Some(4));
        assert_eq!(run_b_err("A=PRGSIZE(4)").errnum(), Some(10));
        assert_eq!(run_b_err("A=PRGSIZE(0,3)").errnum(), Some(10)); // type out of range
        assert_eq!(run_b_err("A=PRGSIZE(0,0,0)").errnum(), Some(4));
    }

    // ---- M6-T5: faithful limitation stubs (XON/MIC/MOTION/MP/DIALOG) ----

    #[test]
    fn xon_mic_enables_then_mic_commands_run() {
        // Without XON MIC the mic commands raise errnum 36 (hw_verified s_t11c).
        assert_eq!(run_b_err("MICSTART 0,0,1").errnum(), Some(36));
        // After XON MIC they run (no real sampler — a faithful no-op).
        let vm = run_b("XON MIC\nMICSTART 0,0,1\nMICSTOP\nPRINT \"OK\"");
        assert_eq!(vm.console_text(), "OK");
        assert!(vm.device.mic);
        // MICDATA reads 0 once the mic is on (live waveform needs hardware).
        let vm = run_b("XON MIC\nV=MICDATA(0)\nPRINT V");
        assert_eq!(vm.console_text(), "0");
        // XOFF MIC disables again.
        let vm = run_b("XON MIC\nXOFF MIC");
        assert!(!vm.device.mic);
    }

    #[test]
    fn xon_motion_enables_then_sensors_run() {
        // Without XON MOTION the sensor reads raise errnum 37 (hw_verified s_t11b).
        assert_eq!(run_b_err("GYROA OUT P,R,Y").errnum(), Some(37));
        assert_eq!(run_b_err("ACCEL OUT X,Y,Z").errnum(), Some(37));
        assert_eq!(run_b_err("GYROSYNC").errnum(), Some(37));
        // After XON MOTION the OUT vars receive zeroed axes (live values need hardware).
        let vm = run_b("XON MOTION\nGYROA OUT P,R,Y\nGYROSYNC\nPRINT P;R;Y");
        assert_eq!(vm.console_text(), "000");
        assert!(vm.device.motion);
    }

    #[test]
    fn xon_expad_sets_result_true() {
        // XON EXPAD sets RESULT to TRUE (1) per the docs; XON MOTION leaves it untouched.
        let vm = run_b("XON EXPAD\nPRINT RESULT");
        assert_eq!(vm.console_text(), "1");
        assert!(vm.device.expad);
    }

    #[test]
    fn dialog_sets_result_and_returns_headless_outcome() {
        // Statement form -> RESULT = 0 (Time out, no user headless).
        let vm = run_b("DIALOG \"hi\"\nPRINT RESULT");
        assert_eq!(vm.console_text(), "0");
        // Confirm function form -> returns 0, RESULT 0.
        let vm = run_b("R=DIALOG(\"ok?\",1)\nPRINT R;RESULT");
        assert_eq!(vm.console_text(), "00");
        // File-name input form (string 2nd arg) -> RESULT -1, empty string.
        let vm = run_b("S$=DIALOG(\"\",\"Name?\")\nPRINT LEN(S$);RESULT");
        assert_eq!(vm.console_text(), "0-1");
        // Too many arguments -> Syntax error (3) (hw_verified, s_t4f).
        assert_eq!(run_b_err("A=DIALOG(\"a\",0,\"b\",0,9)").errnum(), Some(3));
    }

    #[test]
    fn mp_offline_session_reads() {
        // No wireless peers: the whole-session status is 0 and MPSTART leaves RESULT FALSE.
        let vm = run_b("MPSTART 2,\"ROOM\"\nPRINT RESULT;MPSTAT()");
        assert_eq!(vm.console_text(), "00");
        // MPRECV yields no data: sender id -1, empty string.
        let vm = run_b("MPRECV OUT SID,RCV$\nPRINT SID;LEN(RCV$)");
        assert_eq!(vm.console_text(), "-10");
        // A peer-indexed read is out of range with 0 terminals connected.
        assert_eq!(run_b_err("N=MPGET(0,0)").errnum(), Some(10));
        assert_eq!(run_b_err("A$=MPNAME$(0)").errnum(), Some(10));
        // MPSEND/MPEND validate and no-op offline.
        let vm = run_b("MPSEND \"HI\"\nMPEND\nPRINT \"OK\"");
        assert_eq!(vm.console_text(), "OK");
    }

    // ---- Multi-slot: USE / EXEC (M6-T6) ----

    #[test]
    fn use_numeric_marks_a_slot_executable() {
        // hw_verified (use.yaml: USE 1 → ok): a valid non-running slot is marked usable.
        // Boot state: only the running slot 0 is usable.
        let vm = run_b("PRINT \"OK\"");
        assert!(vm.slot_used(0));
        assert!(!vm.slot_used(1));
        let vm = run_b("USE 1:USE 3");
        assert!(vm.slot_used(1));
        assert!(vm.slot_used(3));
        assert!(!vm.slot_used(2));
    }

    #[test]
    fn use_running_slot_is_illegal_function_call() {
        // hw_verified (USE 0 → errnum 4): you cannot USE the slot you are executing.
        assert_eq!(run_b_err("USE 0").errnum(), Some(4));
    }

    #[test]
    fn use_out_of_range_slot_is_out_of_range() {
        // hw_verified (USE -1 / USE 4 → errnum 10).
        assert_eq!(run_b_err("USE -1").errnum(), Some(10));
        assert_eq!(run_b_err("USE 4").errnum(), Some(10));
    }

    #[test]
    fn use_resource_string_running_slot_is_illegal_function_call() {
        // hw_verified (USE "PRG0:X" → errnum 4): the resource form also rejects the running slot.
        assert_eq!(run_b_err("USE \"PRG0:X\"").errnum(), Some(4));
    }

    #[test]
    fn use_resource_string_bad_resource_is_illegal_function_call() {
        // hw_verified: an unknown resource type, a PRG index past the family, or an empty name
        // is Illegal function call (4) — NOT Out of range, unlike the SAVE resolver.
        assert_eq!(run_b_err("USE \"FOO:X\"").errnum(), Some(4));
        assert_eq!(run_b_err("USE \"PRG4:X\"").errnum(), Some(4));
        assert_eq!(run_b_err("USE \"PRG5:X\"").errnum(), Some(4));
        assert_eq!(run_b_err("USE \"PRG1:\"").errnum(), Some(4));
    }

    #[test]
    fn use_resource_string_missing_file_is_load_failed() {
        // hw_verified (USE "PRG1:X" → errnum 46): a valid slot + missing file is Load failed.
        assert_eq!(run_b_err("USE \"PRG1:NOPE\"").errnum(), Some(46));
    }

    #[test]
    fn use_resource_string_existing_file_marks_slot() {
        // A valid slot whose program file exists is marked usable. Program files share the
        // TXT folder, so a SAVE "TXT:" file is visible to the PRGn: resource form.
        let vm = run_b("SAVE \"TXT:LIB\",\"PRINT 1\"\nUSE \"PRG2:LIB\"");
        assert!(vm.slot_used(2));
    }

    #[test]
    fn exec_out_of_range_slot_is_out_of_range() {
        // hw_verified (EXEC -1 / EXEC 4 → errnum 10).
        assert_eq!(run_b_err("EXEC -1").errnum(), Some(10));
        assert_eq!(run_b_err("EXEC 4").errnum(), Some(10));
    }

    #[test]
    fn exec_empty_slot_is_syntax_error() {
        // hw_verified (EXEC 1 on a fresh, empty slot → errnum 3). No program loader exists
        // yet, so every non-running slot is empty.
        assert_eq!(run_b_err("EXEC 1").errnum(), Some(3));
    }

    #[test]
    fn exec_resource_string_bad_resource_is_illegal_function_call() {
        // hw_verified (EXEC "FOO:X" → errnum 4): an unknown resource type.
        assert_eq!(run_b_err("EXEC \"FOO:X\"").errnum(), Some(4));
    }

    #[test]
    fn exec_resource_string_missing_file_is_load_failed() {
        // hw_verified (EXEC "NOEXISTPROG" → errnum 46): a missing program file is Load failed.
        assert_eq!(run_b_err("EXEC \"NOEXISTPROG\"").errnum(), Some(46));
    }

    #[test]
    fn exec_string_prg_slot_loads_from_storage_and_transfers() {
        // documented form 1 (M6-T6): a program SAVE'd to TXT storage and then EXEC'd via
        // `EXEC "PRGn:file"` is loaded from storage, compiled in-VM, and run — control
        // transfers, so the loaded program's output appears.
        let vm = run_b("SAVE \"TXT:PROG\",\"PRINT 1\"\nEXEC \"PRG1:PROG\"");
        assert_eq!(vm.console_text(), "1");
    }

    #[test]
    fn exec_running_slot_restart_reruns_from_top() {
        // `EXEC 0` restarts the running program from the top (corpus "restart the game"
        // idiom, 1DVK34J/TXT/HNZBUS). A DAT file counter persists across the restart (storage
        // is not touched), so the program terminates on the 2nd pass; the global `A` is
        // re-initialised to its declared-type zero each run — a fresh re-run — so it reads 1
        // (not 2) at the end. "XX" on the (un-cleared) console witnesses the two passes.
        let vm = run_b(
            "DIM C[1]\n\
             IF CHKFILE(\"DAT:K\") THEN LOAD\"DAT:K\",C\n\
             C[0]=C[0]+1\n\
             SAVE\"DAT:K\",C\n\
             A=A+1\n\
             ?\"X\";\n\
             IF C[0]>=2 THEN END\n\
             EXEC 0",
        );
        assert_eq!(vm.console_text(), "XX");
        assert_eq!(real(&vm, "A"), 1.0);
    }

    // ---- firmware default graphic pages (GRP4 sprite, GRP5 BG, GRPF font) ----

    #[test]
    fn vm_boots_with_default_sprite_bg_and_font_pages() {
        // SmileBASIC boots with the system sprite sheet on GRP4, the BG sheet on GRP5, and the
        // font on the hidden GRPF page — so the interpreter renders real sprites/tiles/text out
        // of the box. GRP0..GRP3 stay blank (programs draw their own pixels there).
        let vm = run_b("");
        let g = vm.grp();
        let any_opaque = |p: &sb_render::grp::GrpPage| p.pixels.iter().any(|&h| h & 1 != 0);
        assert!(
            any_opaque(&g.pages[4]),
            "GRP4 should hold the default sprite sheet"
        );
        assert!(
            any_opaque(&g.pages[5]),
            "GRP5 should hold the default BG sheet"
        );
        assert!(any_opaque(&g.grpf), "GRPF should hold the default font");
        assert!(
            g.pages[0].pixels.iter().all(|&h| h == 0),
            "GRP0 boots blank"
        );
        // The SPDEF definition table boots loaded with the firmware templates (not the bare
        // 16×16 placeholder for every slot): template 1 = (16,0,...), template 4095 = (192,480,…).
        let t1 = vm.spdef().get(1);
        assert_eq!((t1.u, t1.v), (16, 0), "SPDEF template 1 = firmware default");
        let t4095 = vm.spdef().get(4095);
        assert_eq!((t4095.u, t4095.v, t4095.w, t4095.h), (192, 480, 96, 32));
    }

    #[test]
    fn acls_reloads_the_default_sprite_bg_and_font_pages() {
        // The documented ACLS reset reloads DEFSP/DEFBG + the font (acls.yaml): clobber the
        // sprite and BG pages to a flat fill, ACLS, and every default page comes back exactly.
        let vm = run_b("GPAGE 0,4:GCLS RGB(255,0,0):GPAGE 0,5:GCLS RGB(255,0,0):ACLS");
        let g = vm.grp();
        assert_eq!(
            g.pages[4].pixels,
            sb_render::assets::default_sprite_page().pixels,
            "ACLS restores the sprite sheet (GRP4)"
        );
        assert_eq!(
            g.pages[5].pixels,
            sb_render::assets::default_bg_page().pixels,
            "ACLS restores the BG sheet (GRP5)"
        );
        assert_eq!(
            g.grpf.pixels,
            sb_render::assets::default_font_page().pixels,
            "ACLS restores the font (GRPF)"
        );
    }
}
