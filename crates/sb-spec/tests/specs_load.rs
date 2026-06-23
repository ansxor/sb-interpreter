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

// ── `disassembled`-provenance guardrail ──────────────────────────────────────────────
//
// The confidence ladder is the contract, and `disassembled` is the load-bearing rung:
// it asserts "I read the ARM/VFP handler BODY in the listing", not "I looked up the
// dispatch address and wrote plausible prose from the docs". Commit df691b1 reverted 14
// spec slices that did exactly the latter — they cited the `dispatch` handler address but
// never ran `disasm.py show` to read the body, then labeled docs-inference `disassembled`.
//
// This test makes that fraud mechanically detectable. For every `type: disassembled`
// source ref it requires EVIDENCE OF A BODY READ, and rejects the two tells the reverted
// batch carried. It is honest about its limits (a determined faker can satisfy the shape),
// but it raises the floor from "free text" to "must look like you read the listing", and
// it is the only check in the gate that bites every commit (the 34 MB `.lst` is gitignored
// and absent in CI, so the listing cross-check below is a local-only bonus).

/// True if `needle` occurs in `hay` as a whole word (not inside a longer identifier) —
/// so `mov` matches `mov r0,#0x8` but not `remove`. `hay` must be lowercase.
fn whole_word(hay: &str, needle: &str) -> bool {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i].chars().next_back().map(is_word).unwrap_or(false);
        let after = hay[i + needle.len()..]
            .chars()
            .next()
            .map(is_word)
            .unwrap_or(false);
        !before && !after
    })
}

/// Numeric values of hex tokens in `s`. With `prefix = "0x"` matches `0x…`/`@0x…`; with
/// `prefix = "FUN_"` matches Ghidra `FUN_<hex>` names. Both `FUN_001415a0` and `@0x1415a0`
/// yield `0x1415a0`, so a FUN_ address can be compared against the plain-`0x` address set.
fn hex_values(s: &str, prefix: &str) -> Vec<u64> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    for (idx, _) in s.match_indices(prefix) {
        let mut j = idx + prefix.len();
        let k = j;
        while j < b.len() && b[j].is_ascii_hexdigit() {
            j += 1;
        }
        if j - k >= 4 {
            if let Ok(v) = u64::from_str_radix(&s[k..j], 16) {
                out.push(v);
            }
        }
    }
    out
}

/// The structural rules (R2 + R3) on one `disassembled` ref string. Returns one message
/// per violation (empty = the ref looks like a genuine body read). Pure + listing-free so
/// it can be unit-tested against the actual reverted refs (see `reverted_refs_are_caught`).
fn body_read_violations(id: &str, r: &str) -> Vec<String> {
    // ARM/VFP mnemonics, matched as WHOLE WORDS so they can't hide inside prose
    // ("move"/"remove" don't contain the mnemonic `mov`). The prose-ambiguous ones
    // (`add`/`sub`/`and`/`or`/`b`) are deliberately omitted — when a ref genuinely quotes
    // them it also carries a `#0x` immediate or a second address, which the rules below
    // already credit.
    const MNEMONICS: [&str; 26] = [
        "vmov", "vldr", "vstr", "vcmpe", "vcmp", "vcvt", "vsqrt", "vneg", "vabs", "vmrs", "vmla",
        "ldrb", "strb", "blx", "mov", "movw", "movt", "cmp", "cmn", "tst", "ldr", "orr", "eor",
        "bic", "lsl", "lsr",
    ];
    let rl = r.to_lowercase();
    let mut out = Vec::new();

    // R2 — the dual-address tell: a `FUN_<hex>` whose address never appears as a real
    // `0x<hex>` token. A genuine body read produces consistent addresses; the reverted
    // ATTR ref glued `FUN_0014bec4` to a different `@0x14c090`.
    let addrs: Vec<u64> = hex_values(r, "0x");
    for fun in hex_values(r, "FUN_") {
        if !addrs.contains(&fun) {
            out.push(format!(
                "{id}: disassembled ref cites FUN_…(0x{fun:x}) but that address never \
                 appears as a plain 0x token (mismatched FUN_/@0x — the cited address was \
                 not actually read): {r:.90}"
            ));
        }
    }

    // R3 — evidence of a body read: a quoted ARM immediate (`#0x8`) or whole-word
    // mnemonic, OR ≥2 distinct addresses (handler + an internal site/const/helper), OR a
    // parser-keyword form (verified-not-dispatched, so it has no single handler body).
    let is_parser = rl.contains("parser keyword")
        || rl.contains("not in the builtin dispatch table")
        || rl.contains("special form");
    let has_arm = rl.contains("#0x") || MNEMONICS.iter().any(|m| whole_word(&rl, m));
    let distinct: std::collections::HashSet<u64> = addrs.iter().copied().collect();
    if !(is_parser || has_arm || distinct.len() >= 2) {
        out.push(format!(
            "{id}: disassembled ref shows no evidence of a body read (no ARM/VFP mnemonic, \
             <2 distinct addresses, not a parser-keyword form) — looks like docs-inference \
             cited as `disassembled`. Run `disasm.py show <addr>` and cite real listing \
             detail: {r:.90}"
        ));
    }
    out
}

#[test]
fn disassembled_sources_show_evidence_of_a_body_read() {
    // TEXT segment (handlers live here): runtime base 0x100000 .. 0x2C8000.
    const TEXT: std::ops::Range<u64> = 0x100000..0x2C8000;

    let listing = {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../sb-disassembly/listings/cia_3.6.0.lst");
        p.exists()
            .then(|| std::fs::read_to_string(&p).unwrap_or_default())
    };

    let specs = sb_spec::load_all(&spec_dir()).expect("specs should load");
    let mut failures = Vec::new();
    for s in &specs {
        for src in &s.sources {
            if src.source_type != "disassembled" {
                continue;
            }
            let r = &src.reference;
            failures.extend(body_read_violations(&s.id, r));

            // Local-only cross-check (skipped in CI where the .lst is absent): every
            // TEXT-range address cited must actually occur in the listing.
            if let Some(lst) = &listing {
                for a in hex_values(r, "0x").iter().filter(|&&a| TEXT.contains(&a)) {
                    let needle = format!("{a:08X}");
                    if !lst.contains(&needle) {
                        failures.push(format!(
                            "{}: disassembled ref cites @0x{a:x} but {needle} is not in \
                             cia_3.6.0.lst (fabricated/typo'd handler address): {r:.90}",
                            s.id
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "disassembled-provenance violations ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ── Error-table conformance (S-T14a) ────────────────────────────────────────────────
//
// `spec/reference/errors.yaml` is the errnum→message table, verified against the binary's
// errnum→string pointer array @0x3054f8 (.data, 56 entries, errnum 0..55) feeding the
// formatter FUN_001e94a8. These are frozen goldens: the `.bin` is gitignored/absent in CI,
// so the cross-check against the disassembly happened once (S-T14a) and is replayed here as
// a deterministic fixture. If real SmileBASIC ever disagrees, fix the YAML, not the test.

#[derive(serde::Deserialize)]
struct ErrTable {
    errors: Vec<ErrRow>,
}

#[derive(serde::Deserialize)]
struct ErrRow {
    num: i32,
    name: String,
}

#[test]
fn error_table_matches_disassembly() {
    let path = spec_dir().join("reference/errors.yaml");
    let text = std::fs::read_to_string(&path).expect("errors.yaml readable");
    let table: ErrTable = serde_yaml::from_str(&text).expect("errors.yaml parses");

    // Contiguous errnum 0..=55, in order, no gaps/dupes, every name non-empty.
    assert_eq!(table.errors.len(), 56, "error table must hold errnum 0..55");
    for (i, row) in table.errors.iter().enumerate() {
        assert_eq!(
            row.num, i as i32,
            "error rows must be contiguous and in order"
        );
        assert!(!row.name.is_empty(), "errnum {} has empty name", row.num);
    }

    // Spot-check the frozen errnum→name mapping read from the @0x3054f8 pointer table.
    // Includes the boundaries (0, 55), the oracle-confirmed 10, the two binary-vs-docs
    // wording differences (41, 43), and a binary-only entry the docs omit (48).
    let by_num: std::collections::HashMap<i32, &str> = table
        .errors
        .iter()
        .map(|r| (r.num, r.name.as_str()))
        .collect();
    for (num, name) in [
        (0, "No Error"),
        (2, "Illegal Instruction"),
        (3, "Syntax error"),
        (10, "Out of range"),
        (12, "Out of code memory"),
        (41, "String is too long"),         // docs say "String too long"
        (43, "Can't use from direct mode"), // docs say "...DIRECT mode"
        (4, "Illegal function call"),       // oracle: X=ABS() → errnum 4
        (7, "Divide by zero"),              // oracle: A=1/0   → errnum 7
        (8, "Type mismatch"),               // oracle: S$=5    → errnum 8
        (47, "Illegal MML"),
        (48, "Uninitialized variable used"), // binary-only, not in docs
        (55, "Too many arguments"),
    ] {
        assert_eq!(by_num.get(&num), Some(&name), "errnum {num} message");
    }
}

// ── System-variable table conformance (S-T14b) ───────────────────────────────────────
//
// `spec/reference/sysvars.yaml` is the system-variable table, verified against the binary:
// every name is a UTF-16LE string in the .rodata keyword/name pool (addr recorded per row),
// referenced by the keyword table near @0x2c8e00. These are frozen goldens replayed here as
// a deterministic fixture (the `.bin` is gitignored/absent in CI). The writability split was
// cross-checked against the sbsave corpus; TRUE/FALSE carry their fixed values. If real
// SmileBASIC ever disagrees, fix the YAML, not the test.

#[derive(serde::Deserialize)]
struct SysvarTable {
    system_variables: Vec<SysvarRow>,
}

#[derive(serde::Deserialize)]
struct SysvarRow {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    writable: bool,
    addr: String,
    #[serde(default)]
    value: Option<i64>,
}

#[test]
fn sysvar_table_matches_disassembly() {
    let path = spec_dir().join("reference/sysvars.yaml");
    let text = std::fs::read_to_string(&path).expect("sysvars.yaml readable");
    let table: SysvarTable = serde_yaml::from_str(&text).expect("sysvars.yaml parses");

    // The documented 24-name set, exactly — no additions/drops.
    assert_eq!(
        table.system_variables.len(),
        24,
        "sysvar table must hold the 24 documented system variables"
    );

    // Structural soundness of every row: non-empty name, valid type tag, plausible binary
    // address. TIME$/DATE$ are the only strings; everything else is an Integer.
    for row in &table.system_variables {
        assert!(!row.name.is_empty(), "sysvar row has empty name");
        assert!(
            row.ty == "i" || row.ty == "s",
            "{}: type must be i or s, got {:?}",
            row.name,
            row.ty
        );
        let want_str = row.name.ends_with('$');
        assert_eq!(
            row.ty == "s",
            want_str,
            "{}: string type iff name ends in $",
            row.name
        );
        let a = row
            .addr
            .strip_prefix("0x")
            .and_then(|h| u64::from_str_radix(h, 16).ok())
            .unwrap_or_else(|| panic!("{}: addr not 0x-hex: {}", row.name, row.addr));
        // Names live in the .rodata pool [0x2C8000, 0x2FD000).
        assert!(
            (0x2C8000..0x2FD000).contains(&a),
            "{}: name addr 0x{a:x} outside .rodata pool",
            row.name
        );
    }

    let by_name: std::collections::HashMap<&str, &SysvarRow> = table
        .system_variables
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();

    // Spot-check frozen rows: name → (writable, addr, value). Addresses are the UTF-16LE
    // string locations read from the binary; writability is docs+corpus; TRUE/FALSE values.
    for (name, writable, addr, value) in [
        ("MAINCNT", false, "0x2ef8c4", None),
        ("VERSION", false, "0x2ef5c8", None),
        ("HARDWARE", false, "0x2ef1cc", None),
        ("TABSTEP", true, "0x2ef68c", None), // writable (docs + corpus XCMD.LIB)
        ("SYSBEEP", true, "0x2ef670", None), // writable (docs + corpus, 237 files)
        ("CSRX", false, "0x2efa54", None),   // read-only (corpus: only `CSRX==` compares)
        ("TRUE", false, "0x2ed1f8", Some(1)),
        ("FALSE", false, "0x2ed00c", Some(0)),
        ("TIME$", false, "0x2eead8", None),
        ("DATE$", false, "0x2eeae4", None),
        ("CALLIDX", false, "0x2efa44", None),
    ] {
        let row = by_name
            .get(name)
            .unwrap_or_else(|| panic!("sysvar {name} missing"));
        assert_eq!(row.writable, writable, "{name} writable");
        assert_eq!(row.addr, addr, "{name} addr");
        assert_eq!(row.value, value, "{name} value");
    }

    // Oracle goldens (S-T14b): frozen invariant values for SmileBASIC 3.6.0.
    #[derive(serde::Deserialize)]
    struct OracleBlock {
        oracle: Goldens,
    }
    #[derive(serde::Deserialize)]
    struct Goldens {
        goldens: std::collections::HashMap<String, i64>,
    }
    let ob: OracleBlock = serde_yaml::from_str(&text).expect("oracle block parses");
    assert_eq!(ob.oracle.goldens.get("TRUE"), Some(&1), "oracle TRUE=1");
    assert_eq!(ob.oracle.goldens.get("FALSE"), Some(&0), "oracle FALSE=0");
    assert_eq!(
        ob.oracle.goldens.get("VERSION"),
        Some(&50724864), // &H03060000 → 3.6.0
        "oracle VERSION=&H03060000"
    );
    assert_eq!(
        ob.oracle.goldens.get("CALLIDX"),
        Some(&0),
        "oracle CALLIDX=0"
    );

    // Only TABSTEP and SYSBEEP are writable (the documented + corpus-verified split).
    let writables: std::collections::BTreeSet<&str> = table
        .system_variables
        .iter()
        .filter(|r| r.writable)
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        writables,
        ["SYSBEEP", "TABSTEP"].into_iter().collect(),
        "exactly TABSTEP and SYSBEEP are writable"
    );
}

#[test]
fn reverted_refs_are_caught() {
    // The exact ref shapes from commit df691b1 (the 14 reverted slices) must be flagged.
    // SPSET: handler address + plausible prose, no body detail (FUN_ == @0x, 1 distinct addr).
    let spset = "cia_3.6.0.lst SPSET handler FUN_001415a0 @0x1415a0: registers the sprite \
                 slot and initializes its transform/SPVAR state; OUT-IX forms scan for a \
                 free management number";
    assert!(
        !body_read_violations("SPSET", spset).is_empty(),
        "SPSET-style docs-prose-with-address must be flagged"
    );
    // ATTR: mismatched FUN_/@0x (0x14bec4 vs 0x14c090) — never produced by a real read.
    let attr = "cia_3.6.0.lst ATTR handler FUN_0014bec4/@0x14c090: sets the console \
                character display-attribute state used by the console renderer";
    assert!(
        !body_read_violations("ATTR", attr).is_empty(),
        "ATTR-style mismatched FUN_/@0x must be flagged"
    );
    // A genuine body-read ref (real ACOS spec) must NOT be flagged.
    let acos = "cia_3.6.0.lst ACOS handler @0x149258; argcount!=1 -> errnum 4; range guard \
                vcmpe against -1.0 (0xBFF0000000000000 @0x149348) / 1.0 @0x149350 -> errnum \
                10 when outside [-1,1]; acos computed @0x2988d8";
    assert!(
        body_read_violations("ACOS", acos).is_empty(),
        "a genuine multi-address body-read ref must pass"
    );
}
