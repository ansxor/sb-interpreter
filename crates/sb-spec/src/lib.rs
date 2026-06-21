//! `sb-spec` — load the YAML instruction specs and report coverage.
//!
//! Each `spec/instructions/<stem>.yaml` describes one SmileBASIC instruction at the
//! `documented` confidence layer (auto-generated from `sb-docs/` by
//! `tools/gen_specs.py`). Verified/authored conformance tests and oracle-harvested
//! `expect:` values live in a parallel overlay `spec/tests/<stem>.yaml`, merged here —
//! so regenerating the documented layer never clobbers ground truth.
//!
//! The deterministic conformance suite (milestone M1+) executes each spec's merged
//! `tests` against `sb-core`. The [`coverage`] report counts specs per
//! [`Confidence`] level so "faithful" is measurable.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Where the specs live, relative to the workspace root.
pub const SPEC_DIR: &str = "spec";

/// The confidence ladder. `Ord` follows declaration order:
/// `Documented < Community < Observed < Disassembled < HwVerified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Documented,
    Community,
    Observed,
    Disassembled,
    HwVerified,
}

impl Confidence {
    pub const ALL: [Confidence; 5] = [
        Confidence::Documented,
        Confidence::Community,
        Confidence::Observed,
        Confidence::Disassembled,
        Confidence::HwVerified,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Confidence::Documented => "documented",
            Confidence::Community => "community",
            Confidence::Observed => "observed",
            Confidence::Disassembled => "disassembled",
            Confidence::HwVerified => "hw_verified",
        }
    }
}

/// What kind of language element an instruction is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Statement,
    Function,
    Operator,
    SystemVar,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Arg {
    pub name: String,
    #[serde(default)]
    pub desc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Form {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub args: Vec<Arg>,
    #[serde(default)]
    pub returns: Vec<Arg>,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub confidence: Option<Confidence>,
}

/// Expected result of a conformance test case.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Expect {
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub error: Option<ErrorExpect>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorExpect {
    pub errnum: u8,
}

/// A single conformance test (from the `spec/tests/` overlay).
#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub code: String,
    #[serde(default)]
    pub expect: Expect,
}

/// One instruction's full spec (documented layer + merged test overlay).
#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    pub id: String,
    pub kind: Kind,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub semantics: Vec<String>,
    #[serde(default)]
    pub forms: Vec<Form>,
    #[serde(default)]
    pub see_also: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub confidence: Confidence,
    /// Merged from `spec/tests/<stem>.yaml`; empty if no overlay exists.
    #[serde(default, skip)]
    pub tests: Vec<TestCase>,
}

/// The test overlay file shape (`spec/tests/<stem>.yaml`).
#[derive(Debug, Clone, Default, Deserialize)]
struct TestOverlay {
    #[serde(default)]
    tests: Vec<TestCase>,
}

#[derive(Debug)]
pub enum LoadError {
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, serde_yaml::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(p, e) => write!(f, "I/O error reading {}: {e}", p.display()),
            LoadError::Parse(p, e) => write!(f, "parse error in {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for LoadError {}

/// Load every `spec/instructions/*.yaml`, merging the `spec/tests/` overlay.
/// `spec_dir` is the path to the `spec/` directory.
pub fn load_all(spec_dir: &Path) -> Result<Vec<Spec>, LoadError> {
    let instr_dir = spec_dir.join("instructions");
    let tests_dir = spec_dir.join("tests");

    let mut paths: Vec<PathBuf> = std::fs::read_dir(&instr_dir)
        .map_err(|e| LoadError::Io(instr_dir.clone(), e))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    paths.sort();

    let mut specs = Vec::with_capacity(paths.len());
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|e| LoadError::Io(path.clone(), e))?;
        let mut spec: Spec =
            serde_yaml::from_str(&text).map_err(|e| LoadError::Parse(path.clone(), e))?;

        let stem = path.file_stem().unwrap_or_default();
        let overlay_path = tests_dir.join(stem).with_extension("yaml");
        if overlay_path.exists() {
            let otext = std::fs::read_to_string(&overlay_path)
                .map_err(|e| LoadError::Io(overlay_path.clone(), e))?;
            let overlay: TestOverlay =
                serde_yaml::from_str(&otext).map_err(|e| LoadError::Parse(overlay_path, e))?;
            spec.tests = overlay.tests;
        }
        specs.push(spec);
    }
    Ok(specs)
}

/// Per-confidence counts plus totals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    pub total: usize,
    pub with_tests: usize,
    pub test_count: usize,
    pub by_confidence: Vec<(Confidence, usize)>,
}

pub fn coverage(specs: &[Spec]) -> Coverage {
    let by_confidence = Confidence::ALL
        .iter()
        .map(|&c| (c, specs.iter().filter(|s| s.confidence == c).count()))
        .collect();
    Coverage {
        total: specs.len(),
        with_tests: specs.iter().filter(|s| !s.tests.is_empty()).count(),
        test_count: specs.iter().map(|s| s.tests.len()).sum(),
        by_confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_orders_correctly() {
        assert!(Confidence::Documented < Confidence::HwVerified);
        assert!(Confidence::Disassembled < Confidence::HwVerified);
        assert!(Confidence::Documented < Confidence::Community);
    }
}
