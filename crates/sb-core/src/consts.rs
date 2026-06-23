//! Built-in `#NAME` named constants (button/color/flag literals).
//!
//! SmileBASIC has 79 reserved `#NAME` constants (`#WHITE`, `#UP`, `#L`, `#A`, …). They are
//! NOT runtime variables: the value is substituted inline as an Integer literal wherever the
//! name appears (an expression, a `DATA` item, an argument). The disassembly confirms this —
//! the sorted keyword record for each `#NAME` carries only the name pointer, with no stored
//! value (third field 0), so the constant is folded at compile time, not read from a table at
//! runtime (see `spec/reference/constants.yaml`).
//!
//! The table below is the frozen golden: every value was `hw_verified` via the sb-oracle
//! (`PRINT <#CONST>` on real SmileBASIC 3.6.0, task S-T14c) — including the 7 values where the
//! oracle overrode the docs (`#BLUE`, `#CYAN`, the `#ZL`/`#ZR` swap, `#BGROT90/180/270`).
//! `SmileBASIC` Integer is `i32`, so a color word like `&HFFF8F8F8` (`#WHITE`) is the signed
//! `-460552`. The integration test `tests/constants_table.rs` asserts this table matches
//! `spec/reference/constants.yaml` exactly (name set + per-name signed value), so the two can
//! never drift.
//!
//! Keys are the constant name WITHOUT the leading `#`, ASCII-upper-cased — matching the token
//! the lexer produces ([`crate::token::TokenKind::Const`], which strips the `#` and upper-cases).

/// `(name, value)` for every built-in `#NAME` constant, sorted by name for binary search.
/// `value` is the constant's signed `i32` (the unsigned ARGB/flag word reinterpreted as `i32`).
static CONSTANTS: &[(&str, i32)] = &[
    ("A", 16),
    ("AQUA", -16713480),
    ("B", 32),
    ("BGREVH", 16384),
    ("BGREVV", 32768),
    ("BGROT0", 0),
    ("BGROT180", 8192),
    ("BGROT270", 12288),
    ("BGROT90", 4096),
    ("BLACK", -16777216),
    ("BLUE", -16776968),
    ("CHKC", 64),
    ("CHKI", 8),
    ("CHKR", 16),
    ("CHKS", 32),
    ("CHKUV", 4),
    ("CHKV", 128),
    ("CHKXY", 1),
    ("CHKZ", 2),
    ("CYAN", -16713480),
    ("DOWN", 2),
    ("FALSE", 0),
    ("FUCHSIA", -524040),
    ("GRAY", -8355712),
    ("GREEN", -16744448),
    ("L", 256),
    ("LEFT", 4),
    ("LIME", -16713728),
    ("MAGENTA", -524040),
    ("MAROON", -8388608),
    ("NAVY", -16777088),
    ("NO", 0),
    ("OFF", 0),
    ("OLIVE", -8355840),
    ("ON", 1),
    ("PURPLE", -8388480),
    ("R", 512),
    ("RED", -524288),
    ("RIGHT", 8),
    ("SILVER", -4144960),
    ("SPADD", 32),
    ("SPREVH", 8),
    ("SPREVV", 16),
    ("SPROT0", 0),
    ("SPROT180", 4),
    ("SPROT270", 6),
    ("SPROT90", 2),
    ("SPSHOW", 1),
    ("TBLACK", 1),
    ("TBLUE", 9),
    ("TCYAN", 13),
    ("TEAL", -16744320),
    ("TGRAY", 14),
    ("TGREEN", 4),
    ("TLIME", 5),
    ("TMAGENTA", 11),
    ("TMAROON", 2),
    ("TNAVY", 8),
    ("TOLIVE", 6),
    ("TPURPLE", 10),
    ("TRED", 3),
    ("TREVH", 4),
    ("TREVV", 8),
    ("TROT0", 0),
    ("TROT180", 2),
    ("TROT270", 3),
    ("TROT90", 1),
    ("TRUE", 1),
    ("TTEAL", 12),
    ("TWHITE", 15),
    ("TYELLOW", 7),
    ("UP", 1),
    ("WHITE", -460552),
    ("X", 64),
    ("Y", 128),
    ("YELLOW", -460800),
    ("YES", 1),
    ("ZL", 4096),
    ("ZR", 2048),
];

/// Resolve a built-in `#NAME` constant to its signed `i32` value. `name` is the bare constant
/// name (no `#`), ASCII-upper-cased — exactly what the lexer stores in `TokenKind::Const`.
/// Returns `None` for an unknown name (the caller leaves it as an unresolved reference).
pub fn lookup(name: &str) -> Option<i32> {
    CONSTANTS
        .binary_search_by(|(k, _)| (*k).cmp(name))
        .ok()
        .map(|i| CONSTANTS[i].1)
}

/// The full `(name, value)` table — used only by the drift test in `tests/constants_table.rs`.
#[doc(hidden)]
pub fn all() -> &'static [(&'static str, i32)] {
    CONSTANTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_constants_resolve() {
        assert_eq!(lookup("UP"), Some(1));
        assert_eq!(lookup("L"), Some(256));
        assert_eq!(lookup("R"), Some(512));
        assert_eq!(lookup("A"), Some(16));
        // Color words are the signed i32 of the ARGB value (&HFFF8F8F8 -> -460552).
        assert_eq!(lookup("WHITE"), Some(-460552));
        assert_eq!(lookup("RED"), Some(-524288));
        // Oracle-corrected values (docs were wrong).
        assert_eq!(lookup("BLUE"), Some(-16776968)); // &HFF0000F8
        assert_eq!(lookup("CYAN"), lookup("AQUA")); // #CYAN == #AQUA
        assert_eq!(lookup("ZL"), Some(4096));
        assert_eq!(lookup("ZR"), Some(2048));
    }

    #[test]
    fn unknown_constant_is_none() {
        assert_eq!(lookup("NOPE"), None);
        assert_eq!(lookup(""), None);
        // Lookup is case-sensitive on the already-upper-cased key.
        assert_eq!(lookup("white"), None);
    }

    #[test]
    fn table_is_sorted_and_unique() {
        for w in CONSTANTS.windows(2) {
            assert!(w[0].0 < w[1].0, "table not sorted/unique at {:?}", w[0].0);
        }
    }
}
