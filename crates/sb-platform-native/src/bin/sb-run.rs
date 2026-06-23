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
//! Exit codes:
//!   0  — ran to completion (`END`/fell off the end) or stopped via `STOP`.
//!   1  — a SmileBASIC error (parse/compile/runtime): `ERRNUM`/`ERRLINE` on stderr.
//!   2  — a usage / host error (missing arg, unreadable file).

use std::process::ExitCode;

use sb_core::builtins::StdBuiltins;
use sb_core::compiler::compile_with;
use sb_core::{parse, Vm, VmError};

/// A SmileBASIC error reduced to its `ERRNUM` + 1-based `ERRLINE`, however it was
/// raised (parse, compile or runtime).
#[derive(Debug)]
struct SbError {
    errnum: u32,
    line: u32,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: sb-run <program.sb3>");
        return ExitCode::from(2);
    };

    let src = match std::fs::read_to_string(&path) {
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
