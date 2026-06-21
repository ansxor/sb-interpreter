//! `sb-spec-coverage` — print spec coverage (counts per confidence level).
//!
//! Used in CI to produce the coverage artifact. Run from the workspace root, or pass
//! the path to the `spec/` directory:
//!
//! ```text
//! cargo run -p sb-spec --bin sb-spec-coverage [-- spec]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let spec_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(sb_spec::SPEC_DIR));

    let specs = match sb_spec::load_all(&spec_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let cov = sb_spec::coverage(&specs);
    println!(
        "SmileBASIC 3.6.0 spec coverage ({} instructions)",
        cov.total
    );
    println!("  confidence:");
    for (conf, n) in &cov.by_confidence {
        let pct = if cov.total > 0 {
            100.0 * *n as f64 / cov.total as f64
        } else {
            0.0
        };
        println!("    {:<13} {:>4}  ({:>5.1}%)", conf.label(), n, pct);
    }
    println!(
        "  tests: {} cases across {} instructions ({} still untested)",
        cov.test_count,
        cov.with_tests,
        cov.total - cov.with_tests,
    );
    ExitCode::SUCCESS
}
