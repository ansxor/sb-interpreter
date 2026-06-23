//! `sb-run` — the headless SmileBASIC runner (M1-T11).
//!
//! Loads a `.sb3` file (plain UTF-8 SmileBASIC source), runs it through the
//! `sb-core` pipeline (`lexer → parser → compiler → VM`, see
//! `spec/concepts/execution-model.md`) with no window, and dumps the resulting
//! text console grid to stdout — exactly what the oracle scrapes from console
//! memory. On a SmileBASIC error it prints `ERRNUM`/`ERRLINE` to stderr.
//!
//! This is the executable `harness/diff/replay.py` shells out to
//! (`target/debug/sb-run`) for the deterministic full-program replay gate.
//!
//! With `--grp <out.png>` it instead renders the GRP **display page** (the visible top-left
//! 400×240 crop, RGBA8888) to a PNG and writes it to `out.png` — the deterministic graphics
//! golden the M2-T5 pixel-diff (`harness/diff/replay.py`) compares against an oracle GRP
//! capture (O-T6). The program still runs through the same VM; only the output differs.
//!
//! Exit codes:
//!   0  — ran to completion (`END`/fell off the end) or stopped via `STOP`.
//!   1  — a SmileBASIC error (parse/compile/runtime): `ERRNUM`/`ERRLINE` on stderr.
//!   2  — a usage / host error (missing arg, unreadable file).

use std::process::ExitCode;

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::{parse, Vm, VmError};
use sb_render::compositor::grp_page_to_framebuffer;
use sb_render::grp::{GRP_VISIBLE_HEIGHT, GRP_VISIBLE_WIDTH};

/// A SmileBASIC error reduced to its `ERRNUM` + 1-based `ERRLINE`, however it was
/// raised (parse, compile or runtime).
#[derive(Debug)]
struct SbError {
    errnum: u32,
    line: u32,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `--grp <out.png> <program.sb3>`: render the display page to a golden PNG instead of
    // dumping console text. Everything else is the headless text runner.
    if args.first().map(String::as_str) == Some("--grp") {
        let (Some(out), Some(path)) = (args.get(1), args.get(2)) else {
            eprintln!("usage: sb-run --grp <out.png> <program.sb3>");
            return ExitCode::from(2);
        };
        return run_grp(path, out);
    }

    let Some(path) = args.first() else {
        eprintln!("usage: sb-run <program.sb3>  |  sb-run --grp <out.png> <program.sb3>");
        return ExitCode::from(2);
    };

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sb-run: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };

    match run(&src) {
        Ok(text) => {
            // The console grid is the deterministic stdout. `println!` adds the final
            // newline so piped output ends cleanly even when the last row is full.
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
            ExitCode::from(1)
        }
    }
}

/// Run `path`'s program and write its GRP display page (visible 400×240 crop) as a PNG to
/// `out`. The VM runs to completion (or to its first error — the partial page is still
/// written, mirroring how the windowed host composites a halted scene); a SmileBASIC error is
/// reported on stderr and yields exit 1, but the PNG is produced either way so the diff sees
/// what the program actually drew.
fn run_grp(path: &str, out: &str) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sb-run: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    // parse → compile share the same `{errnum, loc.line}` error shape; collapse either into a
    // pre-run SbError, otherwise run the VM and snapshot its display page.
    let compiled = parse(&src)
        .map_err(|e| SbError {
            errnum: e.errnum,
            line: e.loc.line,
        })
        .and_then(|ast| {
            compile_with(&ast, &StdBuiltins).map_err(|e| SbError {
                errnum: e.errnum,
                line: e.loc.line,
            })
        });
    let (fb, err) = match compiled {
        Ok(program) => {
            let mut vm = Vm::new(program);
            let err = vm.run().err().and_then(sb_error_of);
            let page = &vm.grp().pages[vm.grp().display_page as usize];
            let fb = grp_page_to_framebuffer(
                page,
                GRP_VISIBLE_WIDTH as usize,
                GRP_VISIBLE_HEIGHT as usize,
            );
            (Some(fb), err)
        }
        Err(e) => (None, Some(e)),
    };
    if let Some(fb) = fb {
        if let Err(e) = std::fs::write(out, sb_render::png::encode(&fb)) {
            eprintln!("sb-run: cannot write {out}: {e}");
            return ExitCode::from(2);
        }
    }
    match err {
        None => ExitCode::SUCCESS,
        Some(e) => {
            eprintln!("ERRNUM={} ERRLINE={}", e.errnum, e.line);
            ExitCode::from(1)
        }
    }
}

/// Reduce a runtime [`VmError`] to its `ERRNUM`/`ERRLINE` (or `None` for a non-error halt).
fn sb_error_of(e: VmError) -> Option<SbError> {
    match e {
        VmError::Sb { errnum, line } => Some(SbError { errnum, line }),
        VmError::Unsupported(what) => {
            eprintln!("sb-run: unsupported: {what}");
            Some(SbError { errnum: 0, line: 0 })
        }
        VmError::Assert { message, line } => {
            eprintln!("sb-run: ASSERT__ failed at line {line}: {message}");
            Some(SbError { errnum: 0, line })
        }
    }
}

/// Run `src` to completion, returning the console text or the SmileBASIC error.
fn run(src: &str) -> Result<String, SbError> {
    let ast = parse(src).map_err(|e| SbError {
        errnum: e.errnum,
        line: e.loc.line,
    })?;
    let program = compile_with(&ast, &StdBuiltins).map_err(|e| SbError {
        errnum: e.errnum,
        line: e.loc.line,
    })?;
    let mut vm = Vm::new(program);
    match vm.run() {
        // `END`, falling off the end and `STOP` are all non-error halts.
        Ok(_) => Ok(vm.console_text()),
        Err(VmError::Sb { errnum, line }) => Err(SbError { errnum, line }),
        // An opcode whose handler lands in a later milestone — surface it as a
        // generic "Internal error" (errnum 0) rather than masquerading as success.
        Err(VmError::Unsupported(what)) => {
            eprintln!("sb-run: unsupported: {what}");
            Err(SbError { errnum: 0, line: 0 })
        }
        // A failed `ASSERT__` (test-mode builtin) — report and exit as an error.
        Err(VmError::Assert { message, line }) => {
            eprintln!("sb-run: ASSERT__ failed at line {line}: {message}");
            Err(SbError { errnum: 0, line })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn prints_console_text() {
        let text = run(r#"?"HI""#).expect("ok");
        assert!(text.starts_with("HI"), "got {text:?}");
    }

    #[test]
    fn fizzbuzz_fixture_runs() {
        // The committed full-program fixture the replay gate uses. `console_text`
        // is the final scrolled grid, so we assert it ends on I=100 → BUZZ and
        // still shows the FIZZBUZZ multiples-of-15 line above it.
        let src = include_str!("../../../../harness/corpus/programs/fizzbuzz.sb3");
        let text = run(src).expect("fizzbuzz runs clean");
        assert!(text.contains("FIZZBUZZ"), "got {text:?}");
        assert!(text.trim_end().ends_with("BUZZ"), "got {text:?}");
    }

    #[test]
    fn runtime_error_carries_errnum_and_line() {
        // SQR(-1) → Out of range (10), on source line 1 (hw_verified, O-T5).
        let err = run("A=SQR(-1)").expect_err("should error");
        assert_eq!(err.errnum, 10);
        assert_eq!(err.line, 1);
    }

    #[test]
    fn parse_error_is_syntax_errnum_3() {
        let err = run("FOR I=").expect_err("should error");
        assert_eq!(err.errnum, 3);
    }
}
