//! Canonical SmileBASIC 3.6.0 error table — the `errnum → message` authority.
//!
//! Every error carrier in the interpreter ([`crate::vm::VmError::Sb`],
//! [`crate::parser::ParseError`], [`crate::compiler::CompileError`],
//! [`crate::value::RuntimeError`]) holds only an `errnum: u32`; the human-facing string
//! SmileBASIC actually displays (and that `ERR$`-style surfacing would return) lives here,
//! reproduced **byte-for-byte from the binary's string pool** rather than re-spelled.
//!
//! # Disassembly (the source of truth — not the docs)
//!
//! The errnum→string pointer table is `*[0x3054f8]` in `.data`: 56 word-pointers for
//! errnum `0..=55`, terminated by a `0` word at `0x3054f8 + 56*4`. Each points into the
//! ASCII string pool `[0x2e965c, 0x2e9ac0)` in `.rodata` (runtime addr = `.bin` offset +
//! `0x100000`). All 56 entries below were dereferenced straight out of
//! `SmileBASIC_3.6.0_CIA.bin` (e.g. table[3] → `0x2e9a00` `"Syntax error"`, table[10] →
//! `0x2e97e4` `"Out of range"`, table[55] → `0x2e9a10` `"Too many arguments"`).
//!
//! The message formatter `FUN_001e94a8` indexes it with a range guard:
//! `sub r0,r0,#0x1; cmp r0,#0x37` then `ldrcc r9,[table, r5, lsl #2]` — i.e. it treats
//! `errnum-1` as unsigned and only looks the table up when `errnum-1 < 0x37 (55)`, so a
//! valid display is `errnum ∈ [1, 55]`. Anything else (errnum `0`, or `≥ 56`) takes the
//! `adrcs r9,0x1e9588` branch = the literal `"Internal Error"` fallback. The handler then
//! optionally appends `" (<detail>)"` (parens at `0x1e95a4`/`0x1e95a8`) and a trailing
//! space-prefixed location string, and finally stores the errnum to `*[0x3054e8]`. The bare
//! message — what we reproduce — is exactly the pool string.
//!
//! Note table[0] is `"No Error"` (the cleared-`ERRNUM` state) but the *formatter* never
//! emits it: errnum 0 means "no error", so its display path falls into the `"Internal
//! Error"` fallback. [`error_message`] mirrors the formatter; [`ERROR_NAMES`] exposes the
//! raw pool including index 0.
//!
//! Confidence: `disassembled` (string pool + formatter body read directly). The *names* are
//! verified byte-for-byte; several `desc` notes in `spec/reference/errors.yaml` for the
//! binary-only errnums (0..2, 48..55) remain `community`/oracle-pending.

/// The error string for each `errnum`, indexed `0..=55`, byte-for-byte from the binary's
/// `.rodata` pool via the `*[0x3054f8]` pointer table. Index `0` is the cleared state
/// (`"No Error"`); see [`error_message`] for the displayed (formatter) semantics.
pub const ERROR_NAMES: [&str; 56] = [
    "No Error",                            // 0   0x2e99f4  (cleared ERRNUM; not displayed)
    "Internal Error",                      // 1   0x2e99e4  (also the formatter's fallback)
    "Illegal Instruction",                 // 2   0x2e99c0
    "Syntax error",                        // 3   0x2e9a00
    "Illegal function call",               // 4   0x2e9914
    "Stack overflow",                      // 5   0x2e9a6c
    "Stack underflow",                     // 6   0x2e9a50
    "Divide by zero",                      // 7   0x2e99d4
    "Type mismatch",                       // 8   0x2e98e4
    "Overflow",                            // 9   0x2e9a60
    "Out of range",                        // 10  0x2e97e4
    "Out of memory",                       // 11  0x2e9ab0
    "Out of code memory",                  // 12  0x2e9a9c
    "Out of DATA",                         // 13  0x2e965c
    "Undefined label",                     // 14  0x2e98f4
    "Undefined variable",                  // 15  0x2e9820
    "Undefined function",                  // 16  0x2e9998
    "Duplicate label",                     // 17  0x2e9904
    "Duplicate variable",                  // 18  0x2e9834
    "Duplicate function",                  // 19  0x2e99ac
    "FOR without NEXT",                    // 20  0x2e9750
    "NEXT without FOR",                    // 21  0x2e9724
    "REPEAT without UNTIL",                // 22  0x2e9700
    "UNTIL without REPEAT",                // 23  0x2e9738
    "WHILE without WEND",                  // 24  0x2e9690
    "WEND without WHILE",                  // 25  0x2e96a4
    "THEN without ENDIF",                  // 26  0x2e96ec
    "ELSE without ENDIF",                  // 27  0x2e96d8
    "ENDIF without IF",                    // 28  0x2e96c4
    "DEF without END",                     // 29  0x2e9680
    "RETURN without GOSUB",                // 30  0x2e9668
    "Subscript out of range",              // 31  0x2e97f4
    "Nested DEF",                          // 32  0x2e96b8
    "Can't continue",                      // 33  0x2e9890
    "Illegal symbol string",               // 34  0x2e98a0
    "Illegal file format",                 // 35  0x2e9a24
    "Mic is not available",                // 36  0x2e9848
    "Motion sensor is not available",      // 37  0x2e9860
    "Use PRGEDIT before any PRG function", // 38  0x2e9974
    "Animation is too long",               // 39  0x2e98cc
    "Illegal animation data",              // 40  0x2e9764
    "String is too long",                  // 41  0x2e98b8  (docs: "String too long")
    "Communication buffer overflow",       // 42  0x2e9a7c
    "Can't use from direct mode",          // 43  0x2e97c8  (docs: "Can't use from DIRECT mode")
    "Can't use in program",                // 44  0x2e995c
    "Can't use in tool program",           // 45  0x2e9940
    "Load failed",                         // 46  0x2e977c
    "Illegal MML",                         // 47  0x2e9718
    "Uninitialized variable used",         // 48  0x2e9788
    "Protected resource",                  // 49  0x2e97b4
    "Protected file",                      // 50  0x2e9880
    "DLC not found",                       // 51  0x2e97a4
    "Incompatible statement",              // 52  0x2e9a38
    "END without call",                    // 53  0x2e992c
    "Array is too large",                  // 54  0x2e980c
    "Too many arguments",                  // 55  0x2e9a10
];

/// The message SmileBASIC displays for `errnum`, reproducing the formatter
/// `FUN_001e94a8` exactly: the pool string for a real error `errnum ∈ [1, 55]`, and the
/// `"Internal Error"` fallback for errnum `0` (no error) or any errnum `≥ 56`.
///
/// This is the bare message; the binary may additionally append `" (<detail>)"` and a
/// trailing location, which depend on runtime context and are not modeled here.
pub fn error_message(errnum: u32) -> &'static str {
    if (1..=55).contains(&errnum) {
        ERROR_NAMES[errnum as usize]
    } else {
        // FUN_001e94a8: `cmp (errnum-1),#0x37` fails for 0 and ≥56 -> `adrcs r9,0x1e9588`.
        "Internal Error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Byte-for-byte golden lifted straight from SmileBASIC_3.6.0_CIA.bin: the 56 pointers
    // at *[0x3054f8] dereferenced into the .rodata ASCII pool [0x2e965c, 0x2e9ac0).
    // `disassembled` — this is the binary's own table, not a re-spelling of the docs.
    const POOL_GOLDEN: [&str; 56] = [
        "No Error",
        "Internal Error",
        "Illegal Instruction",
        "Syntax error",
        "Illegal function call",
        "Stack overflow",
        "Stack underflow",
        "Divide by zero",
        "Type mismatch",
        "Overflow",
        "Out of range",
        "Out of memory",
        "Out of code memory",
        "Out of DATA",
        "Undefined label",
        "Undefined variable",
        "Undefined function",
        "Duplicate label",
        "Duplicate variable",
        "Duplicate function",
        "FOR without NEXT",
        "NEXT without FOR",
        "REPEAT without UNTIL",
        "UNTIL without REPEAT",
        "WHILE without WEND",
        "WEND without WHILE",
        "THEN without ENDIF",
        "ELSE without ENDIF",
        "ENDIF without IF",
        "DEF without END",
        "RETURN without GOSUB",
        "Subscript out of range",
        "Nested DEF",
        "Can't continue",
        "Illegal symbol string",
        "Illegal file format",
        "Mic is not available",
        "Motion sensor is not available",
        "Use PRGEDIT before any PRG function",
        "Animation is too long",
        "Illegal animation data",
        "String is too long",
        "Communication buffer overflow",
        "Can't use from direct mode",
        "Can't use in program",
        "Can't use in tool program",
        "Load failed",
        "Illegal MML",
        "Uninitialized variable used",
        "Protected resource",
        "Protected file",
        "DLC not found",
        "Incompatible statement",
        "END without call",
        "Array is too large",
        "Too many arguments",
    ];

    #[test]
    fn names_match_binary_pool_byte_for_byte() {
        assert_eq!(ERROR_NAMES, POOL_GOLDEN);
    }

    #[test]
    fn displayed_message_uses_pool_for_real_errors() {
        // Every real error errnum 1..=55 displays its exact pool string.
        for n in 1..=55u32 {
            assert_eq!(error_message(n), POOL_GOLDEN[n as usize]);
        }
        // A few spot-checks that pin the exact (case-sensitive) spelling SB uses.
        assert_eq!(error_message(3), "Syntax error");
        assert_eq!(error_message(10), "Out of range");
        assert_eq!(error_message(43), "Can't use from direct mode"); // lowercase "direct"
        assert_eq!(error_message(55), "Too many arguments");
    }

    #[test]
    fn out_of_band_errnums_fall_back_to_internal_error() {
        // FUN_001e94a8 range guard: errnum 0 (no error) and errnum >= 56 -> "Internal Error".
        assert_eq!(error_message(0), "Internal Error");
        assert_eq!(error_message(56), "Internal Error");
        assert_eq!(error_message(999), "Internal Error");
        // ...even though the raw pool slot 0 is "No Error".
        assert_eq!(ERROR_NAMES[0], "No Error");
    }
}
