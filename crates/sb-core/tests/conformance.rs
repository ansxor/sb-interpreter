//! M1-T14 — the deterministic **conformance runner**. Replays every committed
//! code→expect fixture through the full `sb-core` pipeline (parse → compile → VM) and
//! asserts each case's `stdout` or `error.errnum`. No emulator, no network, fixed RNG
//! seeds — this is Phase B (see `harness/README.md`), the hermetic gate that runs in CI.
//!
//! Three fixture sources are loaded (all share the v2 `tests:` schema — `{name, code,
//! expect: {stdout | error: {errnum}}}`):
//!
//! 1. **`harness/corpus/cases/*.yaml`** — cross-cutting curated cases.
//! 2. **`spec/tests/*.yaml`** — per-instruction `hw_verified` overlays harvested by the
//!    oracle (O-T8). These use a `tests:` top-level key (matching `harvest.py` output); the
//!    loader accepts both `cases:` and `tests:` (see [`CaseFile`]). Loaded unconditionally —
//!    no `IN_SCOPE_*` gate — so every frozen overlay case is replayed here.
//! 3. **Inline `tests:` from `spec/instructions/*.yaml`** — but only for the categories
//!    `sb-core` actually implements as pure value→`PRINT` builtins/operators in M1:
//!    **Mathematics** and **Strings** (M1-T7), the bit/logic operators `AND/OR/XOR/DIV/MOD`
//!    (M1-T6 / S-T6a), and the implemented **Control** flow (M1-T8 + parser/compiler:
//!    IF/FOR/WHILE/REPEAT/GOTO/GOSUB/ON/… — see `IN_SCOPE_CONTROL`; `CALL`/`COMMON`/`XON`/
//!    `XOFF` are later-milestone and excluded), the array/variable mutation set (`DIM`/`VAR`/
//!    `DATA`/`SORT`/`SWAP`/`INC`/… — see `IN_SCOPE_DATA_ARRAY_CONSOLE`), and the implemented
//!    **Console input/output** output builtins (`PRINT`/`COLOR`/`CLS`/`INKEY$` — see
//!    `IN_SCOPE_CONSOLE`; the `ATTR`/`CHKCHR`/`FONTDEF`/`SCROLL`/`WIDTH` builtins fold in
//!    with their own increments). Specs `sb-core` implements only *partially* contribute
//!    their deterministic cases via `IN_SCOPE_PARTIAL` (per-case exclusion): `LOCATE`'s
//!    range/arg-shape error guards fold in now while its positioned-output cases stay
//!    oracle-pending. These produce a comparable
//!    `console_text()` (or a checkable errnum). Graphics/sprite/BG/etc. instructions are
//!    intentionally out of scope here (their behavior is page/layer state, exercised by the VM
//!    unit tests + corpus cases) and are folded in as their milestones land.
//!
//! Self-checking `ASSERT__` programs are replayed by [`assert_programs_pass`] below —
//! `m1_conformance.sb3` (hand-written) and `otya_m1.sb3` (the real `otya_test.sb3` golden
//! sliced to the M1 feature set; the full file folds in once CALL/DATE$/DTREAD land).
//!
//! [`every_implemented_builtin_spec_is_in_scope`] keeps the wiring honest: it fails if any
//! registered builtin grows a spec with inline tests that isn't folded into a `IN_SCOPE_*`
//! list — so a future milestone can't implement an instruction and silently skip its
//! documented cases.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use sb_core::builtins::{StdBuiltins, BUILTIN_NAMES};
use sb_core::compiler::compile_with;
use sb_core::parser::parse;
use sb_core::vm::{Vm, VmError};

/// Instruction categories whose inline spec tests `sb-core` can replay today (pure
/// value→`PRINT` semantics, deterministic, comparable via `console_text()`).
const IN_SCOPE_CATEGORIES: &[&str] = &["Mathematics", "Strings"];
/// Operators (not categorised as Math/String) that are likewise implemented + comparable.
const IN_SCOPE_OPERATORS: &[&str] = &["AND", "OR", "XOR", "DIV", "MOD"];
/// Control-flow instructions (category `Control`) that `sb-core` implements in M1 (M1-T8 +
/// parser/compiler lowering) and whose inline `tests:` are `PRINT`-comparable. The category
/// is NOT taken wholesale: `XON`/`XOFF` are input toggles (M4) — those fold in with their
/// milestones. `CALL` (dynamic dispatch) and `COMMON` (same-slot `COMMON DEF`) are now in
/// scope (M6-T6): `CALL "name"` resolves a DEF by a runtime name string (`Op::CallDynamic`),
/// so `calls_by_name` (→ stdout) and `undefined_instruction` (→ errnum 16) replay green, and a
/// `COMMON DEF` is invoked just like a plain DEF in its own slot. `USE`/`EXEC` now fold in their
/// hw_verified error/validation model (see `IN_SCOPE_MULTISLOT`); cross-slot COMMON visibility
/// and the EXEC/USE program *transfer* are the remaining M6-T6 multi-program work — queued.) Listed
/// by id.
const IN_SCOPE_CONTROL: &[&str] = &[
    "IF", "THEN", "ELSE", "ELSEIF", "ENDIF", "FOR", "NEXT", "TO", "STEP", "WHILE", "WEND",
    "REPEAT", "UNTIL", "BREAK", "CONTINUE", "GOTO", "GOSUB", "RETURN", "ON", "END", "STOP", "DEF",
    "CALL", "COMMON", "OUT",
];
/// Array / variable **mutation** instructions (`Variables and Arrays` category) that
/// `sb-core` fully implements — including the array-element reference forms (`SWAP A[i],A[j]`,
/// `INC A[i]`, `DEC A[i]`) now that [`Op::PushArrayRef`] is wired (M1-T14 increment). Their
/// inline `tests:` are deterministic + `console_text()`-comparable. `COPY` and `FILL` are now
/// in scope (M1-T14 increment 2026-06-23): COPY copies array→array (`COPY D,S`, dest_offset,
/// src_offset, count forms, 1D auto-extend) or reads a DATA sequence (`COPY D,"@Label"`); FILL
/// overwrites a value into an element range. `VAR` is now in scope: its
/// duplicate-declaration errnum (18) landed (M1-T14 increment 2026-06-23), so its inline
/// `tests:` (incl. the `duplicate_error` 18 case) replay green. `DATA` is now in scope: its
/// items (numbers, strings, const-exprs, `&H` hex, and `#NAME` named constants — the
/// `data_named_const` case `DATA #L` → 256) all parse/fold (M1-T14 increment, `#NAME`
/// resolution via `sb_core::consts`). Still folding in with their own increments: the
/// `Console` LOCATE cursor-positioned scrape — tracked in beads (bd search "S-T5a"). `INPUT`/`LINPUT`
/// are in scope for their *error* inline tests only (the literal-receiver / function-form
/// Syntax error 3, both hw_verified); their read forms block on live input and have no
/// deterministic golden. Listed by id.
const IN_SCOPE_DATA_ARRAY_CONSOLE: &[&str] = &[
    "DIM", "VAR", "DATA", "SORT", "RSORT", "COPY", "FILL", "PUSH", "POP", "SHIFT", "UNSHIFT",
    "SWAP", "INC", "DEC", "INPUT", "LINPUT",
];
/// `Console input/output` instructions whose builtins `sb-core` implements (M1-T8) and whose
/// inline `tests:` are deterministic + `console_text()`-comparable: `PRINT` (formatting),
/// `COLOR` (fg/bg set + range errnums), `CLS` (clears the grid), and `INKEY$` (empty-buffer
/// `""`). The category is NOT taken wholesale by id: `LOCATE` is folded PARTIALLY via
/// `IN_SCOPE_PARTIAL` — its range (→ 10) / arg-shape (→ 4) error guards replay green now;
/// only its *positioned*-output cases (`LOCATE 20,15:PRINT "X"` etc.) stay excluded, scraping
/// to leading-whitespace/`\n`-prefixed text the value-oracle never captured (oracle-pending,
/// see S-T5a / `bd search "S-T5a"`); `ATTR`/`FONTDEF`/`SCROLL`/`WIDTH` builtins are not
/// implemented yet (S-T5c). `CHKCHR` (read a grid cell's UTF-16 code; function only) is in
/// scope (M1-T14 increment 2026-06-23): its empty-cell / out-of-bounds → 0 value cases and
/// its arg-count / statement-use → 4 error cases replay green. Only its `read_printed_char`
/// case is folded PARTIALLY via `IN_SCOPE_PARTIAL` — its setup `PRINT "A";` leaves the glyph
/// on the grid, so the scraped `console_text()` is `"A65"`, not the value-oracle's `"65"`
/// (the CHKCHR read itself is covered by `cases/chkchr.yaml`'s CLS-cleaned round-trip + the
/// console-builtin unit test). Those fold in with their own increments. Listed by id.
const IN_SCOPE_CONSOLE: &[&str] = &[
    "PRINT", "COLOR", "CLS", "INKEY$", "CHKCHR", "ATTR", "SCROLL", "WIDTH", "FONTDEF",
];
/// `Data operations and others` instructions whose semantics `sb-core` implements in M1 and
/// whose inline `tests:` are deterministic + `console_text()`-comparable (M1-T14 increment
/// 2026-06-23): `READ` (walks the DATA cursor — sequential, across-line, float, array-element
/// receiver, out-of-data → 13, type-mismatch → 8), `RESTORE` (label/string-var/computed-label
/// reposition + bare-`RESTORE` type-mismatch → 8), `OPTION` (`STRICT` declared-ok / undeclared
/// → 15, unknown option → 3), and `REM` (line + trailing comment ignored). The rest of the
/// category stays excluded: `WAIT`/`VSYNC` are frame-timing (M4), `DTREAD`/`TMREAD`/`KEY`/
/// `DIALOG` and the `CHK*` builtins aren't implemented yet. Listed by id.
const IN_SCOPE_DATA_OPS: &[&str] = &["READ", "RESTORE", "OPTION", "REM", "WAIT", "VSYNC"];
/// `Graphics` instructions whose builtins `sb-core` implements (M2-T1: the GRP page-state
/// model + color helpers) and whose inline `tests:` are deterministic + `console_text()`-
/// comparable (M1-T14 increment 2026-06-23): `RGB` (channel pack → signed ARGB),
/// `RGBREAD` (unpack via `OUT`), `GPAGE` (display/manip page set+`OUT` get, range errnums),
/// `GCLS` (clear, arg errnums), `GCOLOR` (draw-color set+get), `GPRIO` (priority set, range
/// errnums), and `GCLIP` (clip-rect set, arg errnums), plus the M2-T2 drawing primitives
/// `GPSET`/`GLINE`/`GBOX`/`GFILL`/`GCIRCLE`/`GTRI`/`GPAINT` (whose call-shape / arg-count
/// guards are hw_verified) and `GSPOIT` (read a pixel — now fully in scope, including its
/// `GPSET`-then-read round-trip cases that the M2-T2 drawing primitives enabled). The
/// primitives' *pixel coverage* still has no scalar golden (the framebuffer pixel-diff is
/// O-T6 / M2-T5, queued), so only their shape/error behavior replays here, plus the M2-T3
/// bitmap ops `GCOPY`/`GSAVE`/`GLOAD` (page↔page blit + page↔array transfer — their
/// arg-count / page-range / size / type errnums AND their GSPOIT-readable round-trip pixels
/// and saved element words are all hw_verified, sb-oracle s_t7d; the round-trips live in
/// `harness/corpus/cases/graphics_bitmap.yaml`). Listed by id.
const IN_SCOPE_GRAPHICS: &[&str] = &[
    "RGB", "RGBREAD", "GPAGE", "GCLS", "GCOLOR", "GPRIO", "GCLIP", "GSPOIT", "GPSET", "GLINE",
    "GBOX", "GFILL", "GCIRCLE", "GTRI", "GPAINT", "GCOPY", "GSAVE", "GLOAD",
];
/// `Screen control` instructions whose builtins `sb-core` implements (M1-T8: the console
/// draw-state reset + screen background-color round-trip) and whose inline `tests:` are
/// deterministic + checkable (M1-T14 increment 2026-06-23): `ACLS` (reset console/draw
/// state — 0 or 3 args ok, 1/2 args → errnum 4) and `BACKCOLOR` (set the screen background
/// color; the no-arg statement and the multi-arg form both → errnum 4). The rendered color
/// itself is screen state with no scalar golden, so the assertable behavior is the call-shape
/// / arg-count guard (both hw_verified via sb-oracle batch s_t11d). The display-config
/// commands `XSCREEN`/`DISPLAY`/`VISIBLE` fold in with M4-T4: the screen reconfiguration has
/// no scalar golden, so their assertable behavior is the arg-shape (→ 4) and range (→ 10)
/// guards, all hw_verified via sb-oracle batch s_t11d. Still excluded: `FADE`/`FADECHK`
/// (frame effects, M5/M4 later). Listed by id.
const IN_SCOPE_SCREEN: &[&str] = &[
    "ACLS",
    "BACKCOLOR",
    "XSCREEN",
    "DISPLAY",
    "VISIBLE",
    "FADE",
    "FADECHK",
];
/// `Sprites` instructions whose lifecycle `sb-core` implements (M3-T1: the 512-slot sprite
/// table + create/release/show/hide/query commands) and whose inline `tests:` are
/// deterministic + checkable: `SPSET` (six forms — explicit slot or auto-allocate, range /
/// arg-count errnums hw_verified, OUT/function allocation returning a slot ≥ 0), `SPCLR`
/// (release one/all, range errnum), `SPSHOW`/`SPHIDE` (display toggle + the used-before-SPSET
/// errnum 4 guard, both hw_verified), and `SPUSED` (TRUE=1/FALSE=0, hw_verified). The
/// category is NOT taken wholesale: the transform/collision instructions
/// (`SPOFS`/`SPCHR`/`SPCOL`/…) land with M3-T3 and the visible-render side-effects
/// (a sprite at the right place/orientation) block on the framebuffer oracle (O-T6 / M3-T6,
/// queued). M3-T2's animation/link/vars commands fold in here for their deterministic
/// contract: `SPANIM` (form/argcount/target/data errnums 4/8/10/39/40 — runtime
/// interpolation output is oracle-pending and exercised by sb-render/corpus tests, not the
/// inline spec cases), `SPSTART`/`SPSTOP` (all/one forms + errnums), `SPFUNC` (callback bind
/// — the `CALL SPRITE` dispatch is M3-T6/oracle-pending), `SPVAR` (read/write the 8 internal
/// variables, hw_verified), and `SPLINK`/`SPUNLINK` (parent link + the undocumented
/// `=SPLINK(mgmt)` getter, hw_verified). M3-T3's collision + definition-template commands
/// fold in here for their deterministic contract: `SPCOL` (enable + detection rect/mask,
/// setter forms + the used-before-SPSET/range errnums hw_verified — `OUT`-getter read-back
/// values are oracle-pending), `SPCOLVEC` (movement vector — storage + swept effect read back
/// through SPHITINFO, hw_verified), `SPCHK`
/// (animation-status bitmask — stopped → 0 hw_verified; mid-anim bit values oracle-pending),
/// `SPHITSP`/`SPHITRC` (sprite/rect collision — overlapping/non-overlapping AND swept-approach
/// hit results + errnums hw_verified), `SPHITINFO` (read the hit record — TM=0 default +
/// arg-shape errnums + the swept-collision coordinate/velocity values, position+velocity*time,
/// hw_verified), and
/// `SPDEF` (definition templates — define/read/reset/copy + the W,H=16/attr=1 defaults and
/// the U+W>512 range error hw_verified; non-default field read-back oracle-pending). Listed
/// by id.
const IN_SCOPE_SPRITES: &[&str] = &[
    "SPSET",
    "SPCLR",
    "SPSHOW",
    "SPHIDE",
    "SPUSED",
    "SPANIM",
    "SPSTART",
    "SPSTOP",
    "SPFUNC",
    "SPVAR",
    "SPLINK",
    "SPUNLINK",
    "SPOFS",
    "SPSCALE",
    "SPROT",
    "SPCOLOR",
    "SPHOME",
    "SPCHR",
    "SPPAGE",
    "SPCLIP",
    "SPCOL",
    "SPCOLVEC",
    "SPCHK",
    "SPHITSP",
    "SPHITRC",
    "SPHITINFO",
    "SPDEF",
];
/// BG core specs `sb-core` implements in M3-T4 (the background-tilemap commands). Each is in
/// scope for its deterministic contract: the argument-count / return-shape / layer-range /
/// cell-range / area-limit / tile-size error guards (hw_verified sb-oracle 2026-06-22,
/// s_t9a/b/c/e), the `BGPAGE`/`BGGET` read-back values (deterministic over the tilemap +
/// page state), and the `BGROT` mod-360 normalization. The on-screen *side effects* — the
/// rendered tint, scroll/rotate/scale pixels, and clip area — block on the BG framebuffer
/// oracle (O-T6 / M3-T6, queued); none of the inline spec cases assert those (they expect an
/// empty `stdout` or an errnum). `BGANIM`/`BGCOORD`/`BGCOPY`/`BGLOAD`/`BGSAVE`/… land with
/// M3-T5. Listed by id.
const IN_SCOPE_BG: &[&str] = &[
    "BGSCREEN", "BGPAGE", "BGPUT", "BGGET", "BGFILL", "BGCLR", "BGOFS", "BGROT", "BGSCALE",
    "BGCOLOR", "BGSHOW", "BGHIDE", "BGHOME", "BGCLIP", "BGANIM", "BGSTART", "BGSTOP", "BGCHK",
    "BGVAR", "BGFUNC", "BGCOPY", "BGCOORD", "BGLOAD", "BGSAVE",
];
/// `Various kinds of input` instructions whose builtins `sb-core` implements (M4-T1: the
/// hardware-input snapshot) and whose inline `tests:` are deterministic + checkable: `BUTTON`
/// (the four feature masks — the no-input baseline 0 hw_verified; the feature-ID 0..3 range
/// guard → errnum 10 hw_verified; live button magnitudes need input injection, queued),
/// `STICK`/`STICKEX` (the centred (0,0) baseline + the exactly-2-OUT-var guard → errnum 4
/// hw_verified; live axis magnitudes need hardware, queued), and `BREPEAT` (the 1-or-3 arg
/// gate, the 10/13 reserved-ID → errnum 4 and the ≥14 / negative-start → errnum 10 guards,
/// all hw_verified; the repeat *timing* surfaces only through `BUTTON` feature 1 under live
/// input, exercised by the `input.rs` scripted-timeline unit tests, not the inline cases).
/// Listed by id.
/// M4-T2 adds `TOUCH` (the no-touch STTM=0 baseline + the empty-OUT-slot form, both
/// deterministic for a headless interpreter; the exactly-3-OUT-var guard → errnum 4 is
/// hw_verified — live touch coordinates need input injection, queued) and `KEY` (the 1..5
/// range guard → errnum 10 and the non-string-value → errnum 8 guard, both hw_verified; the
/// undocumented `KEY()` function-form value is oracle-pending, exercised by VM unit tests).
const IN_SCOPE_INPUT: &[&str] = &["BUTTON", "STICK", "STICKEX", "BREPEAT", "TOUCH", "KEY"];
/// `Sound` BGM commands whose transport `sb-core` implements (M5-T3: the registered-tune
/// table + per-track playing/volume/internal-variable state over `AudioState`). Each is in
/// scope for its deterministic contract: the disassembled argument-count / return-shape /
/// track (0..7) / volume (0..127) / tune (0..42|128..255) / variable (0..7) range guards, the
/// MML-compile error (`BGMSET` malformed MML → errnum 47), `BGMVAR`'s write-vs-read form
/// selection (stopped read → -1), and `BGMCHK`'s stopped → 0 boolean. The *audible* output of
/// playback has no deterministic emulator golden (O-T7), so none of the inline spec cases
/// assert it (they expect empty `stdout`, a `0`/`-1` value, or an errnum). `BGMSETD`'s whole
/// contract is now fully in scope (M7-T2 hw_verified, harness/harvest/out/bgmsetd.tsv): its
/// accepted forms (literal / string-var / bare-label-literal / channel-prefix labels), its
/// tune-range (→ 10) / non-string-label (→ 8) / arg-shape (→ 4) / illegal-MML (→ 47) guards,
/// AND the resolved open question — a **missing** label (`BGMSETD 128,"@NOPE"`, no matching
/// `DATA` block) returns NOERR, registering an empty tune rather than raising Undefined label
/// (errnum 14). `sb-core` was corrected to match. Listed by id.
/// The SFX / voice commands (M5-T4) extend the in-scope set: `BEEP` (preset sound effect),
/// `TALK`/`TALKCHK`/`TALKSTOP` (speech transport), `EFCSET`/`EFCON`/`EFCOFF`/`EFCWET` (the
/// music effector over `EffectState`), and `WAVSET`/`WAVSETA` (user MML instruments
/// `@224`–`@255`). Each is in scope for its disassembled arg-shape / range / type contract;
/// the *audible* output has no deterministic golden (O-T7), so the inline cases assert only
/// empty `stdout`, a `0`/`1` boolean (`TALKCHK`), or an errnum. `TALKCHK`'s
/// `bare_statement_syntax_error` case is folded PARTIALLY (excluded below): a bare
/// `TALKCHK()` statement is rejected at parse-time with errnum 3 on real SB, but `sb-core`
/// does not yet track function-vs-statement kind, so the handler raises errnum 4 instead
/// (function-as-statement parse rejection is a broader feature — queued in
/// `bd search "bareword"`).
const IN_SCOPE_SOUND: &[&str] = &[
    "BGMPLAY", "BGMSTOP", "BGMCHK", "BGMVAR", "BGMVOL", "BGMSET", "BGMSETD", "BGMCLEAR", "BEEP",
    "TALK", "TALKCHK", "TALKSTOP", "EFCSET", "EFCON", "EFCOFF", "EFCWET", "WAVSET", "WAVSETA",
];
/// The file commands (M6-T2): `SAVE`/`LOAD`/`FILES`/`DELETE`/`RENAME`/`CHKFILE` (category
/// `Files`) + `PROJECT` (category `DIRECT mode`), over the VM-owned `Storage` (M6-T1). Listed
/// by id rather than category because other `Files`/`DIRECT mode` specs (e.g. `RUN`/`LIST`)
/// are not yet implemented (`USE`/`EXEC` → `IN_SCOPE_MULTISLOT`, `PRG*` → `IN_SCOPE_PRG`). Each spec's inline cases are the
/// hw_verified arg-shape (→ 3/4) / type (→ 8) / DIRECT-only (→ 44) guards plus the
/// `PROJECT=v` variable form; the data-round-trip behaviour is exercised by
/// `harness/corpus/cases/files.yaml`.
const IN_SCOPE_FILES: &[&str] = &[
    "SAVE", "LOAD", "FILES", "DELETE", "RENAME", "CHKFILE", "PROJECT",
];
/// Multi-slot program control (M6-T6): `USE` (mark a slot executable) + `EXEC` (load/run
/// another slot). Each spec's inline cases are the hw_verified error/validation model
/// (2026-06-23): numeric slot out of range → 10; the running slot / a bad resource string →
/// 4; a missing program file → 46; `EXEC` of an empty slot → Syntax error 3. The actual
/// control transfer (loading a compiled program into a slot, switching the running program,
/// cross-slot DEF/label/variable scoping) is the remaining multi-program model — not
/// body-readable in the disassembly, oracle-pending, exercised by `vm.rs` unit tests as
/// `VmError::Unsupported` rather than faked.
const IN_SCOPE_MULTISLOT: &[&str] = &["USE", "EXEC"];
/// The source-code-manipulation family (M6-T4): `PRGEDIT` selects the edit target, the four
/// current-line mutators (`PRGGET$`/`PRGSET`/`PRGINS`/`PRGDEL`) act on it, and `PRGNAME$`/
/// `PRGSIZE` report a slot's file name / counts, over the VM-owned program-slot source. Each
/// spec's inline cases are the hw_verified arg-shape (→ 4) / slot-or-type range (→ 10) /
/// count-0 (→ 10) / no-PRGEDIT cold-state (→ 38) guards. The edited line *content* + the
/// line/char/free counts have no scalar golden in a warm session (oracle-pending), so the
/// round-trip behaviour is exercised by `vm.rs` unit tests, not the inline spec cases.
const IN_SCOPE_PRG: &[&str] = &[
    "PRGEDIT", "PRGGET$", "PRGSET", "PRGINS", "PRGDEL", "PRGNAME$", "PRGSIZE",
];
/// The faithful "limitation stub" family (M6-T5): the microphone (`MIC*`), motion sensors
/// (`GYRO*`/`ACCEL`), wireless multiplayer (`MP*`) and `DIALOG`, plus the `XON`/`XOFF` feature
/// gate. None of the underlying hardware exists in the headless interpreter, so each spec is
/// in scope for its *observable* contract: the disassembled arg-shape (→ 4) / range (→ 10) /
/// type (→ 8) / syntax (→ 3) guards and the XON-MIC / XON-MOTION availability errors (36/37),
/// all hw_verified via the oracle (s_t11b/c, 2026-06-23). The *live* device output (waveform
/// samples, sensor axes, peer payloads, the interactive DIALOG return) has no headless golden
/// (oracle-pending), so no inline case asserts it — the cases expect only an errnum (or, for
/// the reachable forms, are absent). Listed by id (the categories also hold not-yet-scoped
/// specs). `XON`/`XOFF` carry no inline tests, so they fold in as a no-op.
const IN_SCOPE_DEVICE: &[&str] = &[
    "XON", "XOFF", "MICSTART", "MICSTOP", "MICDATA", "MICSAVE", "GYROA", "GYROV", "GYROSYNC",
    "ACCEL", "MPSTART", "MPEND", "MPSET", "MPSTAT", "MPSEND", "MPRECV", "MPGET", "MPNAME$",
    "DIALOG",
];
/// Specs `sb-core` implements only **partially** in M1: each is in scope, but the named
/// cases listed here are EXCLUDED because they block on a later milestone or the
/// console-text oracle. Everything else in the spec — the deterministic, hw_verified
/// arg-count / range / out-of-bounds error guards — replays green today (M1-T14 increment
/// 2026-06-23). `LOCATE`: `basic_xy` now folds in with a `console_text()`-aware expect
/// (15 leading empty rows + the positioned X); `x_edge_50_ok` stays excluded because
/// column-50 is the off-screen right edge and the wrap/no-display behavior is
/// oracle-pending (S-T5a, `bd search "S-T5a"`). (`GSPOIT` is now fully in scope — the M2-T2
/// drawing primitives enabled its three `GPSET`-then-read round-trip cases, so it moved to
/// `IN_SCOPE_GRAPHICS`.) `CHKCHR`: `read_printed_char` now folds in with the
/// harness scrape `"A65"` (the setup glyph stays on the grid); its empty-cell/OOB/arg-count
/// cases fold in now. Tuples are `(spec id, &[excluded case names])`.
const IN_SCOPE_PARTIAL: &[(&str, &[&str])] = &[
    ("LOCATE", &["x_edge_50_ok"]),
    ("TALKCHK", &["bare_statement_syntax_error"]),
    // S-T3a (hw_verified sb-oracle 2026-06-26): real SB 3.6.0 accepts these IF forms but the
    // sb-core parser currently raises errnum 3. The SPEC + oracle evidence is frozen; these
    // cases are excluded only until the parser implements them (queued, bd search "S-T3a parser").
    (
        "IF",
        &[
            "goto_omitted_then_true", // IF cond THEN @label  — bare-label GOTO-omission
            "goto_omitted_then_false",
            "goto_keyword_true", // IF cond GOTO @label
            "goto_keyword_false",
            "endif_rejoin_after_then", // ENDIF then fall-through to next statement
            "endif_rejoin_after_else",
        ],
    ),
    (
        "ELSEIF",
        &[
            "else_if_spaced_nested_y", // ELSE IF (spaced) nested IF, own ENDIF
            "else_if_spaced_nested_z",
            "multiline_else_if_spaced", // multi-line nested IF inside an ELSE clause
        ],
    ),
];

#[derive(Debug, Deserialize)]
struct CaseFile {
    // `harness/corpus/cases/*.yaml` use `cases:`; the oracle-harvested `spec/tests/*.yaml`
    // overlays (and `harvest.py` output) use `tests:`. Accept both so the overlays are
    // actually replayed by the gate (without the alias they parse to zero cases and silently
    // skip — M7-T2 run 4).
    #[serde(default, alias = "tests")]
    cases: Vec<Case>,
}

/// A `spec/instructions/<id>.yaml` document (only the fields the runner needs).
#[derive(Debug, Deserialize)]
struct SpecFile {
    id: String,
    category: Option<String>,
    #[serde(default)]
    tests: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    code: String,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
struct Expect {
    stdout: Option<String>,
    error: Option<ErrorExpect>,
}

#[derive(Debug, Deserialize)]
struct ErrorExpect {
    errnum: u32,
}

/// Run a case's code, returning either its console text (`Ok`) or the SmileBASIC errnum it
/// raised at parse / compile / run time (`Err`).
fn run_case(code: &str) -> Result<String, u32> {
    let ast = parse(code).map_err(|e| e.errnum)?;
    let program = compile_with(&ast, &StdBuiltins).map_err(|e| e.errnum)?;
    let mut vm = Vm::new(program);
    match vm.run() {
        Ok(_) => Ok(vm.console_text()),
        Err(VmError::Sb { errnum, .. }) => Err(errnum),
        Err(other) => panic!("unexpected non-SB VM error: {other:?}"),
    }
}

/// Repo root (two levels up from this crate's manifest).
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `*.yaml` file directly under `dir` (sorted for stable test ordering). A missing
/// directory yields an empty list (e.g. `spec/tests/` before the first oracle harvest).
fn yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out
}

/// Check one case against the runner's result, pushing a human-readable line to `fails`
/// (and nothing on success). `src` labels the fixture file in failure messages.
fn check(case: &Case, src: &str, fails: &mut Vec<String>) {
    let got = run_case(&case.code);
    match (&case.expect.stdout, &case.expect.error) {
        (Some(expected), None) => match got {
            Ok(out) if &out == expected => {}
            Ok(out) => fails.push(format!(
                "{src} `{}` ({}): want stdout {expected:?}, got {out:?}",
                case.name, case.code
            )),
            Err(errnum) => fails.push(format!(
                "{src} `{}` ({}): want stdout {expected:?}, got errnum {errnum}",
                case.name, case.code
            )),
        },
        (None, Some(err)) => match got {
            Err(errnum) if errnum == err.errnum => {}
            Err(errnum) => fails.push(format!(
                "{src} `{}` ({}): want errnum {}, got errnum {errnum}",
                case.name, case.code, err.errnum
            )),
            Ok(out) => fails.push(format!(
                "{src} `{}` ({}): want errnum {}, got stdout {out:?}",
                case.name, case.code, err.errnum
            )),
        },
        _ => fails.push(format!(
            "{src} `{}`: expect must be exactly one of stdout/error",
            case.name
        )),
    }
}

/// Load the curated code→expect case files (`harness/corpus/cases/` + `spec/tests/`).
fn case_files() -> Vec<(String, CaseFile)> {
    let root = root();
    let dirs = [root.join("harness/corpus/cases"), root.join("spec/tests")];
    let mut files = Vec::new();
    for dir in &dirs {
        for path in yaml_files(dir) {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let file: CaseFile = serde_yaml::from_str(&text)
                .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            files.push((name, file));
        }
    }
    files
}

#[test]
fn corpus_and_overlay_cases_pass() {
    let mut fails = Vec::new();
    let mut count = 0usize;
    for (name, file) in case_files() {
        for case in &file.cases {
            check(case, &name, &mut fails);
            count += 1;
        }
    }
    assert!(
        fails.is_empty(),
        "{}/{} curated case(s) failed:\n{}",
        fails.len(),
        count,
        fails.join("\n")
    );
}

/// The single source of truth for "does the M1 `sb-core` implement this spec well enough to
/// replay its inline `tests:` in the hermetic gate?" — used by both
/// [`in_scope_instruction_specs_pass`] (which runs them) and
/// [`every_implemented_builtin_spec_is_in_scope`] (which proves nothing implemented falls
/// through the cracks). A spec is in scope if its category is wholly implemented, its id is
/// one of the individually-listed control/data/console/graphics/screen instructions, or it is
/// a PARTIAL spec (in scope save for a named subset of cases).
fn spec_in_scope(id: &str, category: Option<&str>) -> bool {
    category.is_some_and(|c| IN_SCOPE_CATEGORIES.contains(&c))
        || IN_SCOPE_OPERATORS.contains(&id)
        || IN_SCOPE_CONTROL.contains(&id)
        || IN_SCOPE_DATA_ARRAY_CONSOLE.contains(&id)
        || IN_SCOPE_CONSOLE.contains(&id)
        || IN_SCOPE_DATA_OPS.contains(&id)
        || IN_SCOPE_GRAPHICS.contains(&id)
        || IN_SCOPE_SCREEN.contains(&id)
        || IN_SCOPE_SPRITES.contains(&id)
        || IN_SCOPE_BG.contains(&id)
        || IN_SCOPE_INPUT.contains(&id)
        || IN_SCOPE_SOUND.contains(&id)
        || IN_SCOPE_FILES.contains(&id)
        || IN_SCOPE_MULTISLOT.contains(&id)
        || IN_SCOPE_PRG.contains(&id)
        || IN_SCOPE_DEVICE.contains(&id)
        || IN_SCOPE_PARTIAL.iter().any(|(pid, _)| *pid == id)
}

#[test]
fn in_scope_instruction_specs_pass() {
    let dir = root().join("spec/instructions");

    let mut fails = Vec::new();
    let mut count = 0usize;
    let mut files = 0usize;
    for path in yaml_files(&dir) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let spec: SpecFile =
            serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        // Partial specs: in scope, but a named subset of cases is excluded (blocked on a
        // later milestone / the console-text oracle — see `IN_SCOPE_PARTIAL`).
        let excluded: &[&str] = IN_SCOPE_PARTIAL
            .iter()
            .find(|(id, _)| *id == spec.id.as_str())
            .map(|(_, cases)| *cases)
            .unwrap_or(&[]);
        if !spec_in_scope(&spec.id, spec.category.as_deref()) {
            continue;
        }
        files += 1;
        let src = format!("{}.yaml", spec.id);
        for case in &spec.tests {
            if excluded.contains(&case.name.as_str()) {
                continue;
            }
            check(case, &src, &mut fails);
            count += 1;
        }
    }
    // Guard against the loader silently matching nothing (a moved dir / renamed category).
    assert!(
        files >= 40 && count >= 250,
        "expected the Math+String+operator spec suite (got {files} files, {count} cases)"
    );
    assert!(
        fails.is_empty(),
        "{}/{} in-scope spec case(s) failed:\n{}",
        fails.len(),
        count,
        fails.join("\n")
    );
}

/// Replay each self-checking `ASSERT__` program: it must run to completion with no failed
/// assertion (the `ASSERT__` builtin halts the VM with [`VmError::Assert`] on a false
/// condition — M1-T14).
#[test]
fn assert_programs_pass() {
    let programs = [
        root().join("harness/corpus/programs/m1_conformance.sb3"),
        // The real otya_test.sb3 golden, sliced to the M1-implemented feature set (the
        // CALL/DATE$/TIME$/DTREAD blocks are removed — they land in M3/M6, after which the
        // full file folds in here; see PRD.md M1-T14 and the fixture's header comment).
        root().join("harness/corpus/programs/otya_m1.sb3"),
    ];
    for path in programs {
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let ast = parse(&src).unwrap_or_else(|e| {
            panic!(
                "{}: parse errnum {} at line {}",
                path.display(),
                e.errnum,
                e.loc.line
            )
        });
        let program = compile_with(&ast, &StdBuiltins)
            .unwrap_or_else(|e| panic!("{}: compile errnum {}", path.display(), e.errnum));
        let mut vm = Vm::new(program);
        match vm.run() {
            Ok(_) => {}
            Err(VmError::Assert { message, line }) => {
                panic!(
                    "{}: ASSERT__ failed at line {line}: {message}",
                    path.display()
                )
            }
            Err(e) => panic!("{}: unexpected VM error: {e:?}", path.display()),
        }
    }
}

/// Wiring guard (M1-T14): every registered builtin that has a spec carrying inline `tests:`
/// MUST be folded into the conformance gate (i.e. [`spec_in_scope`] returns true for it).
/// This makes the runner self-policing — when a later milestone implements a new builtin
/// (say M2's `GLINE`) and adds it to [`BUILTIN_NAMES`] but forgets to add its spec id to an
/// `IN_SCOPE_*` list, this test fails, forcing the fold so the new instruction's documented
/// cases actually run. Builtins with no spec, or a spec with no inline tests, are skipped
/// (nothing to replay); `ASSERT__` is a test-only builtin with no spec. The set is empty
/// today — every M1-implemented builtin's spec tests already replay green — so the guard's
/// job is to keep it empty as the surface grows.
#[test]
fn every_implemented_builtin_spec_is_in_scope() {
    let dir = root().join("spec/instructions");
    // Spec id -> (category, has inline tests).
    let mut specs: BTreeMap<String, (Option<String>, bool)> = BTreeMap::new();
    for path in yaml_files(&dir) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let spec: SpecFile =
            serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        specs.insert(
            spec.id.clone(),
            (spec.category.clone(), !spec.tests.is_empty()),
        );
    }

    let mut gaps: Vec<&str> = Vec::new();
    for &name in BUILTIN_NAMES {
        if name == "ASSERT__" {
            continue; // test-only builtin, no spec
        }
        let Some((category, has_tests)) = specs.get(name) else {
            continue; // no spec for this builtin (yet) — nothing to fold
        };
        if !has_tests {
            continue; // spec exists but has no inline cases to replay
        }
        if !spec_in_scope(name, category.as_deref()) {
            gaps.push(name);
        }
    }

    assert!(
        gaps.is_empty(),
        "registered builtin(s) whose spec carries inline tests but is NOT folded into the \
         conformance gate — add each id to an IN_SCOPE_* list (or IN_SCOPE_PARTIAL) in \
         conformance.rs so its documented cases actually run: {gaps:?}"
    );
}
