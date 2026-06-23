//! M1-T5 acceptance: the compiler must turn every real-world corpus program that
//! *parses* into bytecode without panicking.
//!
//! `harness/corpus/sbsave/files/<KEY>/TXT/<NAME>` holds thousands of decoded,
//! real-world SmileBASIC programs (see `harness/corpus/sbsave/README.md`). They are
//! test *inputs*, not goldens: we only assert that compilation never panics (a panic
//! fails the test). Programs the parser rejects are skipped — that is M1-T3's
//! concern; a `CompileError` here is also fine (it is a normal, non-panicking
//! result). The sweep is deterministic and offline (no emulator, no network).

use std::fs;
use std::path::{Path, PathBuf};

use sb_core::{compile, parse};

/// Locate `harness/corpus/sbsave/files` relative to this crate, if present.
fn corpus_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/corpus/sbsave/files")
        .canonicalize()
        .ok()?;
    root.is_dir().then_some(root)
}

/// Collect every `*/TXT/*` source file under the corpus root.
fn collect_txt(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_txt(&path, out);
        } else if path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n == "TXT")
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

#[test]
fn compiles_corpus_without_panicking() {
    let Some(root) = corpus_root() else {
        eprintln!("corpus not present; skipping compile sweep");
        return;
    };

    let mut files = Vec::new();
    collect_txt(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected corpus TXT files under {root:?}"
    );

    let mut parsed = 0usize;
    let mut compiled = 0usize;
    for path in &files {
        let Ok(src) = fs::read_to_string(path) else {
            continue; // non-UTF-8 / unreadable entries are not programs
        };
        let Ok(ast) = parse(&src) else {
            continue; // parser-level gaps are M1-T3's concern
        };
        parsed += 1;
        // The assertion is simply that this call does not panic.
        if compile(&ast).is_ok() {
            compiled += 1;
        }
    }

    // Sanity: the sweep actually exercised a substantial body of real programs.
    assert!(
        parsed > 100,
        "expected to parse many corpus programs, got {parsed}"
    );
    eprintln!(
        "corpus compile sweep: {} files, {} parsed, {} compiled OK",
        files.len(),
        parsed,
        compiled
    );
}
