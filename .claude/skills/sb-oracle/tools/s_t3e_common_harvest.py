#!/usr/bin/env python3
"""Harvest COMMON cross-slot DEF behavior (sb-interpreter-cj8 / S-T3e).

Setup: pre-write a helper program "Q" into slot-1 extdata (on-disk TQ), then run a slot-0
program that LOAD "PRG1:Q",0 : USE 1 : <op> : SAVE result.

Cases:
  - cross_slot_common_callable:       COMMON DEF in slot 1 callable from slot 0 via CALL after USE
  - cross_slot_common_args_and_out:   value args bind, OUT results return to caller's slot-0 var
  - cross_slot_common_returns_value:  value-returning COMMON DEF resolves cross-slot
  - cross_slot_call_without_use:      loaded-but-not-USE'd slot -> Undefined function (16)
  - cross_slot_non_common_def_private: plain (non-COMMON) DEF not visible cross-slot -> 16
  - cross_slot_common_globals_isolated: COMMON DEF reads its own slot's globals, not caller's
  - cross_slot_call_restores_caller:  after return, slot-0 execution resumes against slot-0 state
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sb_extdata as X
import sb_window as W
import run_case as R


def harvest_case(name, helper_src, main_src, result_name="O"):
    """Pre-write helper to slot-1 extdata, then run main_src (slot 0) which LOADs it."""
    if helper_src:
        X.write_file("Q", helper_src if helper_src.endswith("\n") else helper_src + "\n", "TXT")
    v = R.run_program(main_src, result_name=result_name, ftype="TXT")
    if v is None:
        # Halted before SAVE: read ERRNUM/ERRLINE via F2.
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


# ── helper programs (slot 1) ────────────────────────────────────────────────────────────
# NOTE on the function form: real SB 3.6.0 has NO `V=CALL("name",args)` value-returning
# function form (errnum 3 Syntax). A DEF returns a value to its caller only via OUT params,
# OR is called directly as V=F(x) when the name is known at compile time. CALL is the runtime
# by-name STATEMENT form and uses OUT to return. So every value-returning COMMON case here
# uses OUT, not RETURN-in-CALL.
HELPER_SHOUT = 'COMMON DEF SHOUT\nPRINT "HEY"\nEND\n'
HELPER_ADD3 = 'COMMON DEF ADD3 A,B OUT C\nC=A+B+3\nEND\n'
HELPER_PRIV = 'DEF PRIV\nPRINT "NO"\nEND\n'                      # plain (non-COMMON) DEF
HELPER_DOUB = 'COMMON DEF DOUB N OUT C\nC=N*2\nEND\n'           # value-returning COMMON DEF (OUT)
HELPER_NOTE = 'COMMON DEF NOTE\nPRINT "IN";\nEND\n'              # no newline (trailing ;)
HELPER_GLOB = 'G=111\nCOMMON DEF GREAD OUT C\nC=G\nEND\n'        # reads its own slot's G via OUT

# ── main programs (slot 0) ──────────────────────────────────────────────────────────────
MAIN_CALLABLE = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'CALL "SHOUT"\n'
    'SAVE"TXT:O","__DONE__"\n'
)
MAIN_ARGS_OUT = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'CALL "ADD3",10,20 OUT R\n'
    'SAVE"TXT:O",STR$(R)\n'
)
MAIN_RETURNS_VALUE = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'CALL "DOUB",21 OUT D\n'
    'SAVE"TXT:O",STR$(D)\n'
)
MAIN_NO_USE = (
    'LOAD "PRG1:Q",0\n'
    'CALL "SHOUT"\n'
    'SAVE"TXT:O","__DONE__"\n'
)
MAIN_NON_COMMON = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'CALL "PRIV"\n'
    'SAVE"TXT:O","__DONE__"\n'
)
MAIN_GLOB_ISOLATED = (
    'LOAD "PRG1:Q",0\n'
    'G=999\n'
    'USE 1\n'
    'CALL "GREAD" OUT V\n'
    'SAVE"TXT:O",STR$(V)+","+STR$(G)\n'
)
MAIN_RESTORES_CALLER = (
    'LOAD "PRG1:Q",0\n'
    'X=7\n'
    'USE 1\n'
    'CALL "NOTE"\n'
    'PRINT X\n'
    'SAVE"TXT:O","__DONE__"\n'
)


CASES = [
    ("cross_slot_common_callable",        HELPER_SHOUT, MAIN_CALLABLE),
    ("cross_slot_common_args_and_out",     HELPER_ADD3,  MAIN_ARGS_OUT),
    ("cross_slot_common_returns_value",   HELPER_DOUB,  MAIN_RETURNS_VALUE),
    ("cross_slot_call_without_use",        HELPER_SHOUT, MAIN_NO_USE),
    ("cross_slot_non_common_def_private", HELPER_PRIV,  MAIN_NON_COMMON),
    ("cross_slot_common_globals_isolated", HELPER_GLOB,  MAIN_GLOB_ISOLATED),
    ("cross_slot_call_restores_caller",   HELPER_NOTE,  MAIN_RESTORES_CALLER),
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
