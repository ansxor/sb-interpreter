//! Deterministic check that whatever specs exist load and are well-formed.
//!
//! The doc-only generated specs were deleted (see PRD: spec build-out is now a
//! first-class, multi-source milestone). This test no longer asserts a fixed count;
//! it guards that every committed spec parses and is structurally sound, and that the
//! reference tables are present. As the spec suite is rebuilt from docs + disassembly +
//! oracle, the per-instruction `tests` execute against `sb-core` (wired in M-impl).

use std::path::PathBuf;

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

#[test]
fn committed_specs_load_and_are_wellformed() {
    let specs = sb_spec::load_all(&spec_dir()).expect("specs should load");
    for s in &specs {
        assert!(!s.id.is_empty(), "spec has empty id");
        assert!(
            s.summary.is_some() || !s.signatures.is_empty(),
            "{} has neither summary nor signatures",
            s.id
        );
        // Every load-bearing test must declare an expectation.
        for t in &s.tests {
            assert!(
                t.expect.stdout.is_some() || t.expect.value.is_some() || t.expect.error.is_some(),
                "{}: test '{}' has no expectation",
                s.id,
                t.name
            );
        }
    }
}

#[test]
fn reference_tables_present() {
    let dir = spec_dir().join("reference");
    for f in ["errors.yaml", "sysvars.yaml", "constants.yaml"] {
        assert!(dir.join(f).exists(), "missing reference table: {f}");
    }
}
