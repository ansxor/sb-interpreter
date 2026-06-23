//! Replay the cross-cutting deterministic cases in `harness/corpus/cases/basics.yaml`
//! through the full pipeline (parse → compile → VM), asserting each case's `stdout` or
//! `error.errnum`. These are mostly `PRINT` cases, so this is the M1-T8 acceptance check
//! ("corpus/cases/basics.yaml passes") and a standing conformance fixture the deterministic
//! gate replays without the emulator. (The general spec/tests conformance runner is M1-T14.)

use serde::Deserialize;
use std::path::PathBuf;

use sb_core::compiler::compile;
use sb_core::parser::parse;
use sb_core::vm::{Vm, VmError};

#[derive(Debug, Deserialize)]
struct CaseFile {
    cases: Vec<Case>,
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
    let program = compile(&ast).map_err(|e| e.errnum)?;
    let mut vm = Vm::new(program);
    match vm.run() {
        Ok(_) => Ok(vm.console_text()),
        Err(VmError::Sb { errnum, .. }) => Err(errnum),
        Err(other) => panic!("unexpected non-SB VM error: {other:?}"),
    }
}

#[test]
fn basics_cases_pass() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/corpus/cases/basics.yaml");
    let text = std::fs::read_to_string(&path).expect("read basics.yaml");
    let file: CaseFile = serde_yaml::from_str(&text).expect("parse basics.yaml");

    for case in &file.cases {
        let got = run_case(&case.code);
        match (&case.expect.stdout, &case.expect.error) {
            (Some(expected), None) => match got {
                Ok(out) => assert_eq!(
                    &out, expected,
                    "case `{}` ({}): stdout mismatch",
                    case.name, case.code
                ),
                Err(errnum) => panic!(
                    "case `{}` ({}): expected stdout {:?}, got errnum {}",
                    case.name, case.code, expected, errnum
                ),
            },
            (None, Some(err)) => match got {
                Err(errnum) => assert_eq!(
                    errnum, err.errnum,
                    "case `{}` ({}): errnum mismatch",
                    case.name, case.code
                ),
                Ok(out) => panic!(
                    "case `{}` ({}): expected errnum {}, got stdout {:?}",
                    case.name, case.code, err.errnum, out
                ),
            },
            _ => panic!(
                "case `{}`: expect must be exactly one of stdout/error",
                case.name
            ),
        }
    }
}
