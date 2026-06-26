#!/usr/bin/env python3
"""Harvest COPY slot-qualified DATA label form (sb-interpreter-kbv / S-T4c).

The prior 2026-06-26 s_t4c harvest got errnum 14 for COPY A,"1:@L" because slot 1 had no
program loaded AND USE 1 was never issued. Per RESTORE docs: "reference a label from a
different SLOT using RESTORE \"1:@Label\" — the target SLOT should be enabled beforehand
with USE, e.g., USE 1." COPY form-2 shares this slot-qualifier semantics.

Setup: pre-write a helper program "Q" into slot-1 extdata (on-disk TQ), then run a slot-0
program that LOAD "PRG1:Q",0 : USE 1 : COPY A,"1:@L" : SAVE result.

Each case is a (name, helper_src, main_src, expect_kind) tuple. We pre-write the helper,
then run_program(main_src) which writes main to slot 0 (TP), LOADs PRG0:P, RUNs.
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sb_extdata as X
import sb_window as W
import run_case as R


def harvest_case(name, helper_src, main_src, result_name="O"):
    """Pre-write helper to slot-1 extdata, then run main_src (slot 0) which LOADs it.
    If the program halts (no result file), tap F2 to read ERRNUM/ERRLINE."""
    # Pre-write helper program "Q" -> on-disk TQ, loadable via LOAD"PRG1:Q"
    X.write_file("Q", helper_src if helper_src.endswith("\n") else helper_src + "\n", "TXT")
    v = R.run_program(main_src, result_name=result_name, ftype="TXT")
    if v is None:
        # Halted before SAVE: read ERRNUM/ERRLINE via F2 (set by the error halt).
        R._delete_result(result_name)
        W.press("F2")
        import time
        time.sleep(R.RUN_SETTLE)
        W.confirm_dialogs()
        err = R._read_result(result_name)
        if err is not None:
            f = err.strip().split("\t")
            num = f[0] if f and f[0].lstrip("-").isdigit() else None
            line = f[1] if len(f) > 1 and f[1].lstrip("-").isdigit() else None
            v = f"errnum={num} errline={line}" if num else f"halted(no errnum read): {err!r}"
        else:
            v = "halted(no F2 read)"
    print(f"{name}\t{v}", flush=True)
    return v


# ── cases ──────────────────────────────────────────────────────────────────────────────
# Helper Q: @L with DATA 10,20,30
HELPER = "@L\nDATA 10,20,30\n"

# Case 1: COPY A,"1:@L" reads slot-1 DATA into A. Expect "102030".
MAIN_BASIC = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'DIM A[3]\n'
    'COPY A,"1:@L"\n'
    'SAVE"TXT:O",STR$(A[0])+STR$(A[1])+STR$(A[2])\n'
)

# Case 2: with dest_offset. COPY A,"1:@L",2 reads first 2 items. Expect "10300" -> wait, A[0..4]?
# DIM A[3], COPY A,"1:@L" fills 3. For count test: COPY A,"1:@L",2 -> A[0]=10,A[1]=20,A[2]=0.
MAIN_COUNT = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'DIM A[3]\n'
    'COPY A,"1:@L",2\n'
    'SAVE"TXT:O",STR$(A[0])+STR$(A[1])+STR$(A[2])\n'
)

# Case 3: without USE 1 — does COPY still work? (test whether USE is required)
MAIN_NO_USE = (
    'LOAD "PRG1:Q",0\n'
    'DIM A[3]\n'
    'COPY A,"1:@L"\n'
    'SAVE"TXT:O",STR$(A[0])+STR$(A[1])+STR$(A[2])\n'
)

# Case 4: slot-qualified but undefined label in slot 1 -> errnum 14?
# Helper has @L only; ask for @NOPE.
MAIN_UNDEF = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'DIM A[3]\n'
    'COPY A,"1:@NOPE"\n'
    'SAVE"TXT:O",STR$(A[0])\n'
)

# Case 5: slot-qualified but slot 1 has NO program loaded (no LOAD) -> errnum 14?
MAIN_NO_LOAD = (
    'USE 1\n'
    'DIM A[3]\n'
    'COPY A,"1:@L"\n'
    'SAVE"TXT:O",STR$(A[0])\n'
)

# Case 6: string DATA cross-slot. Helper @S with DATA "x","y","z".
HELPER_S = "@S\nDATA \"x\",\"y\",\"z\"\n"
MAIN_STR = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'DIM S$[3]\n'
    'COPY S$,"1:@S"\n'
    'SAVE"TXT:O",S$[0]+S$[1]+S$[2]\n'
)


CASES = [
    ("slot_no_load",    "",       MAIN_NO_LOAD),   # FIRST: slot 1 empty (no prior LOAD)
    ("slot_basic",      HELPER,   MAIN_BASIC),
    ("slot_count2",     HELPER,   MAIN_COUNT),
    ("slot_string",     HELPER_S, MAIN_STR),
    ("slot_no_use",     HELPER,   MAIN_NO_USE),
    ("slot_undef",      HELPER,   MAIN_UNDEF),
]


def main():
    if not R.ready():
        sys.exit("ORACLE NOT READY")
    outpath = sys.argv[1] if len(sys.argv) > 1 else None
    out = open(outpath, "a") if outpath else None
    for name, helper, main_src in CASES:
        v = harvest_case(name, helper, main_src)
        if out:
            out.write(f"{name}\t{v}\n")
            out.flush()
    if out:
        out.close()


if __name__ == "__main__":
    main()
