//! Deterministic check that the whole spec corpus loads and is well-formed.
//! (The execution of each spec's `tests` against `sb-core` arrives in M1.)

use std::path::PathBuf;

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

#[test]
fn all_specs_load_and_are_wellformed() {
    let specs = sb_spec::load_all(&spec_dir()).expect("specs should load");

    // We generated 248 SmileBASIC 3 instruction pages.
    assert_eq!(specs.len(), 248, "expected 248 instruction specs");

    for s in &specs {
        assert!(!s.id.is_empty(), "spec has empty id");
        // Every documented instruction should have at least a summary or a form.
        assert!(
            s.summary.is_some() || !s.forms.is_empty(),
            "{} has neither summary nor forms",
            s.id
        );
    }

    // FLOOR is the showcase: documented + a merged test overlay.
    let floor = specs
        .iter()
        .find(|s| s.id == "FLOOR")
        .expect("FLOOR present");
    assert_eq!(floor.kind, sb_spec::Kind::Function);
    assert!(
        !floor.tests.is_empty(),
        "FLOOR test overlay should be merged"
    );
    let str_err = floor
        .tests
        .iter()
        .find(|t| t.expect.error.is_some())
        .expect("FLOOR has an error-expecting test");
    // The famous one: FLOOR(\"x\") is Type mismatch = errnum 8 (NOT 20).
    assert_eq!(str_err.expect.error.as_ref().unwrap().errnum, 8);
}

#[test]
fn coverage_is_all_documented_for_now() {
    let specs = sb_spec::load_all(&spec_dir()).expect("specs should load");
    let cov = sb_spec::coverage(&specs);
    let documented = cov
        .by_confidence
        .iter()
        .find(|(c, _)| *c == sb_spec::Confidence::Documented)
        .map(|(_, n)| *n)
        .unwrap_or(0);
    // Until oracle harvest runs, everything sits at the documented layer.
    assert_eq!(documented, cov.total);
}
