//! Drift guard: the baked `#NAME` constant table in `sb_core::consts` must match the
//! hw_verified golden in `spec/reference/constants.yaml` EXACTLY — same name set, same signed
//! `i32` value (the `bits` u32 reinterpreted as `i32`). `sb-core` can't read the YAML at
//! runtime (it must build I/O-free for wasm32), so the table is hand-baked; this test (a
//! dev-only fixture, NOT in the wasm build) keeps the two from silently diverging.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ConstantsFile {
    constants: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    name: String,
    bits: u64,
}

fn golden() -> BTreeMap<String, i32> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/reference/constants.yaml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: ConstantsFile =
        serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse constants.yaml: {e}"));
    doc.constants
        .into_iter()
        .map(|r| {
            let key = r
                .name
                .strip_prefix('#')
                .expect("constant name starts with #")
                .to_ascii_uppercase();
            // The golden is the unsigned ARGB/flag word reinterpreted as a signed i32.
            (key, r.bits as u32 as i32)
        })
        .collect()
}

#[test]
fn baked_table_matches_constants_yaml() {
    let golden = golden();
    let baked: BTreeMap<String, i32> = sb_core::consts::all()
        .iter()
        .map(|(k, v)| ((*k).to_string(), *v))
        .collect();

    let mut problems = Vec::new();
    for (name, gv) in &golden {
        match baked.get(name) {
            Some(bv) if bv == gv => {}
            Some(bv) => problems.push(format!("#{name}: golden {gv}, baked {bv}")),
            None => problems.push(format!("#{name}: in constants.yaml but MISSING from table")),
        }
    }
    for name in baked.keys() {
        if !golden.contains_key(name) {
            problems.push(format!("#{name}: in table but NOT in constants.yaml"));
        }
    }

    assert_eq!(
        golden.len(),
        79,
        "expected 79 constants in the golden, found {}",
        golden.len()
    );
    assert!(
        problems.is_empty(),
        "constant table drift vs spec/reference/constants.yaml:\n{}",
        problems.join("\n")
    );
}
