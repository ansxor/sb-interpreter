//! M7-T1 acceptance: the fuzzer's promoted findings replay deterministically in CI.
//!
//! `harness/fuzz/generator.py` is a seeded, spec-signature-driven SmileBASIC program
//! generator. The differential campaign (`harness/diff/run.py`) runs its output through
//! `sb-core` (and, offline, the oracle); any program that made `sb-core` *panic* — a host
//! crash SmileBASIC itself would never do — is a bug. Three were found and fixed (see the
//! per-file repros under `regressions/`); the generated seed sets give ongoing breadth.
//!
//! This test is the hermetic gate (no emulator, no network): it asserts every committed
//! fuzz program runs (or compiles) through the pipeline **without panicking**. A
//! `ParseError`/`CompileError`/`VmError` is a perfectly fine, non-panicking outcome — only
//! an actual Rust panic (e.g. an integer-overflow or a non-char-boundary slice) fails it.
//!
//!   * `regressions/*` + `safe/*` — runtime-safe, GUARANTEED-TERMINATING programs (the safe
//!     generator profile: math/string/bit + bounded FOR only). These are RUN through the VM,
//!     so they exercise the runtime paths the panics lived in (GTRI/GCOPY/VAL, …).
//!   * `broad/*` — the broad profile additionally emits arbitrary command statements that may
//!     not terminate (e.g. `VSYNC <huge>`), so these are only PARSED + COMPILED, never run.

use std::fs;
use std::path::{Path, PathBuf};

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::parser::parse;
use sb_core::vm::Vm;

/// `harness/corpus/fuzz` relative to this crate.
fn fuzz_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/corpus/fuzz")
}

/// Every `*.sb3` directly under `dir`, sorted for stable ordering.
fn sb3_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|x| x == "sb3").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Parse → compile → RUN. The assertion is simply that none of these panic; any error
/// result is fine. Returns whether the program reached the VM (parsed + compiled).
fn run_no_panic(src: &str) -> bool {
    let Ok(ast) = parse(src) else { return false };
    let Ok(program) = compile_with(&ast, &StdBuiltins) else {
        return false;
    };
    let mut vm = Vm::new(program);
    let _ = vm.run(); // any Ok/Err is acceptable; a panic fails the test
    true
}

/// Parse → compile only (for the broad profile, which may not terminate).
fn compile_no_panic(src: &str) {
    let Ok(ast) = parse(src) else { return };
    let _ = compile_with(&ast, &StdBuiltins); // panic = test failure
}

#[test]
fn fuzz_regressions_and_safe_seeds_run_without_panicking() {
    let root = fuzz_root();
    let mut ran = 0usize;
    for sub in ["regressions", "safe"] {
        for path in sb3_files(&root.join(sub)) {
            let src = fs::read_to_string(&path).expect("readable fuzz program");
            if run_no_panic(&src) {
                ran += 1;
            }
        }
    }
    // The committed regressions (gtri/gcopy/val overflow + char-boundary) must reach the VM.
    assert!(
        ran >= 3,
        "expected the runtime fuzz corpus to execute (got {ran})"
    );
}

#[test]
fn fuzz_broad_seeds_compile_without_panicking() {
    let dir = fuzz_root().join("broad");
    let files = sb3_files(&dir);
    assert!(
        !files.is_empty(),
        "expected broad fuzz programs under {dir:?}"
    );
    for path in files {
        let src = fs::read_to_string(&path).expect("readable fuzz program");
        compile_no_panic(&src);
    }
}
