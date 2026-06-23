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
//!    oracle (O-T8). None exist yet; the loader is ready for when they land.
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
//!    oracle-pending; `GSPOIT`'s OOB/arg-count guards fold in now while its `GPSET`
//!    round-trip cases wait on M2-T2. These produce a comparable
//!    `console_text()` (or a checkable errnum). Graphics/sprite/BG/etc. instructions are
//!    intentionally out of scope here (their behavior is page/layer state, exercised by the VM
//!    unit tests + corpus cases) and are folded in as their milestones land.
//!
//! Self-checking `ASSERT__` programs are replayed by [`assert_programs_pass`] below —
//! `m1_conformance.sb3` (hand-written) and `otya_m1.sb3` (the real `otya_test.sb3` golden
//! sliced to the M1 feature set; the full file folds in once CALL/DATE$/DTREAD land).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use sb_core::builtins::StdBuiltins;
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
/// is NOT taken wholesale: `CALL`/`COMMON` are dynamic-dispatch / multi-slot (M6), and
/// `XON`/`XOFF` are input toggles (M4) — those fold in with their milestones. Listed by id.
const IN_SCOPE_CONTROL: &[&str] = &[
    "IF", "THEN", "ELSE", "ELSEIF", "ENDIF", "FOR", "NEXT", "TO", "STEP", "WHILE", "WEND",
    "REPEAT", "UNTIL", "BREAK", "CONTINUE", "GOTO", "GOSUB", "RETURN", "ON", "END", "STOP", "DEF",
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
/// `Console` LOCATE cursor-positioned scrape — queued in `HARVEST_QUEUE.md`. `INPUT`/`LINPUT`
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
/// see S-T5a / `HARVEST_QUEUE.md`); `ATTR`/`CHKCHR`/
/// `FONTDEF`/`SCROLL`/`WIDTH` builtins are not implemented yet (S-T5c). Those fold in with
/// their own increments. Listed by id.
const IN_SCOPE_CONSOLE: &[&str] = &["PRINT", "COLOR", "CLS", "INKEY$"];
/// `Data operations and others` instructions whose semantics `sb-core` implements in M1 and
/// whose inline `tests:` are deterministic + `console_text()`-comparable (M1-T14 increment
/// 2026-06-23): `READ` (walks the DATA cursor — sequential, across-line, float, array-element
/// receiver, out-of-data → 13, type-mismatch → 8), `RESTORE` (label/string-var/computed-label
/// reposition + bare-`RESTORE` type-mismatch → 8), `OPTION` (`STRICT` declared-ok / undeclared
/// → 15, unknown option → 3), and `REM` (line + trailing comment ignored). The rest of the
/// category stays excluded: `WAIT`/`VSYNC` are frame-timing (M4), `DTREAD`/`TMREAD`/`KEY`/
/// `DIALOG` and the `CHK*` builtins aren't implemented yet. Listed by id.
const IN_SCOPE_DATA_OPS: &[&str] = &["READ", "RESTORE", "OPTION", "REM"];
/// `Graphics` instructions whose builtins `sb-core` implements (M2-T1: the GRP page-state
/// model + color helpers) and whose inline `tests:` are deterministic + `console_text()`-
/// comparable (M1-T14 increment 2026-06-23): `RGB` (channel pack → signed ARGB),
/// `RGBREAD` (unpack via `OUT`), `GPAGE` (display/manip page set+`OUT` get, range errnums),
/// `GCLS` (clear, arg errnums), `GCOLOR` (draw-color set+get), `GPRIO` (priority set, range
/// errnums), and `GCLIP` (clip-rect set, arg errnums). The category is NOT taken wholesale:
/// `GSPOIT` (read a pixel) is folded PARTIALLY via `IN_SCOPE_PARTIAL` — its OOB-returns-0
/// and arg-count → 4 guards replay green now; only its `GPSET`-then-read round-trip cases
/// wait on the drawing primitives (M2-T2). The rest of the Graphics category isn't
/// implemented yet. Listed by id.
const IN_SCOPE_GRAPHICS: &[&str] = &[
    "RGB", "RGBREAD", "GPAGE", "GCLS", "GCOLOR", "GPRIO", "GCLIP",
];
/// `Screen control` instructions whose builtins `sb-core` implements (M1-T8: the console
/// draw-state reset + screen background-color round-trip) and whose inline `tests:` are
/// deterministic + checkable (M1-T14 increment 2026-06-23): `ACLS` (reset console/draw
/// state — 0 or 3 args ok, 1/2 args → errnum 4) and `BACKCOLOR` (set the screen background
/// color; the no-arg statement and the multi-arg form both → errnum 4). The rendered color
/// itself is screen state with no scalar golden, so the assertable behavior is the call-shape
/// / arg-count guard (both hw_verified via sb-oracle batch s_t11d). The rest of the category
/// stays excluded: `DISPLAY`/`FADE`/`FADECHK`/`VISIBLE`/`XSCREEN` are display-config / frame
/// effects (M4) and aren't implemented yet. Listed by id.
const IN_SCOPE_SCREEN: &[&str] = &["ACLS", "BACKCOLOR"];
/// Specs `sb-core` implements only **partially** in M1: each is in scope, but the named
/// cases listed here are EXCLUDED because they block on a later milestone or the
/// console-text oracle. Everything else in the spec — the deterministic, hw_verified
/// arg-count / range / out-of-bounds error guards — replays green today (M1-T14 increment
/// 2026-06-23). `LOCATE`: its two *positioned-output* cases (`basic_xy`, `x_edge_50_ok`)
/// scrape to leading-whitespace / newline-prefixed text the value-oracle never captured —
/// oracle-pending (S-T5a, `HARVEST_QUEUE.md`); its range (51,0 / 0,30 / 0,0,2000 → 10) and
/// arg-shape (single-arg / as-function → 4) cases fold in now. `GSPOIT`: its three
/// `GPSET`-then-read round-trip cases need the drawing primitives (`GPSET`, M2-T2); its
/// OOB-returns-0 and arg-count → 4 cases fold in now. Tuples are `(spec id, &[excluded
/// case names])`.
const IN_SCOPE_PARTIAL: &[(&str, &[&str])] = &[
    ("LOCATE", &["basic_xy", "x_edge_50_ok"]),
    (
        "GSPOIT",
        &[
            "roundtrip_red_truncates",
            "roundtrip_white_equals_const",
            "roundtrip_green_top5",
        ],
    ),
];

#[derive(Debug, Deserialize)]
struct CaseFile {
    #[serde(default)]
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

#[test]
fn in_scope_instruction_specs_pass() {
    let dir = root().join("spec/instructions");
    let in_scope_cats: BTreeSet<&str> = IN_SCOPE_CATEGORIES.iter().copied().collect();
    let in_scope_ops: BTreeSet<&str> = IN_SCOPE_OPERATORS.iter().copied().collect();
    let in_scope_control: BTreeSet<&str> = IN_SCOPE_CONTROL.iter().copied().collect();
    let in_scope_dac: BTreeSet<&str> = IN_SCOPE_DATA_ARRAY_CONSOLE.iter().copied().collect();
    let in_scope_console: BTreeSet<&str> = IN_SCOPE_CONSOLE.iter().copied().collect();
    let in_scope_data_ops: BTreeSet<&str> = IN_SCOPE_DATA_OPS.iter().copied().collect();
    let in_scope_graphics: BTreeSet<&str> = IN_SCOPE_GRAPHICS.iter().copied().collect();
    let in_scope_screen: BTreeSet<&str> = IN_SCOPE_SCREEN.iter().copied().collect();

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
        let in_scope = spec
            .category
            .as_deref()
            .is_some_and(|c| in_scope_cats.contains(c))
            || in_scope_ops.contains(spec.id.as_str())
            || in_scope_control.contains(spec.id.as_str())
            || in_scope_dac.contains(spec.id.as_str())
            || in_scope_console.contains(spec.id.as_str())
            || in_scope_data_ops.contains(spec.id.as_str())
            || in_scope_graphics.contains(spec.id.as_str())
            || in_scope_screen.contains(spec.id.as_str())
            || IN_SCOPE_PARTIAL
                .iter()
                .any(|(id, _)| *id == spec.id.as_str());
        if !in_scope {
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
