#!/usr/bin/env python3
"""Harvest CALL function-form vs statement-form cross-slot (sb-interpreter-cj8 / S-T3e).

Investigates whether `V=CALL("name",args)` (function form, returning a value) is accepted
by real SB 3.6.0, vs the statement form `CALL "name",args OUT V`.
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sb_extdata as X
import sb_window as W
import run_case as R


def harvest_case(name, helper_src, main_src, result_name="O"):
    if helper_src:
        X.write_file("Q", helper_src if helper_src.endswith("\n") else helper_src + "\n", "TXT")
    v = R.run_program(main_src, result_name=result_name, ftype="TXT")
    if v is None:
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


HELPER_DBL = 'COMMON DEF DBL N\nRETURN N*2\nEND\n'

# Function form: V=CALL("DBL",21)  -- sb-core accepts, does real SB?
MAIN_FUNC_FORM = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'V=CALL("DBL",21)\n'
    'SAVE"TXT:O",STR$(V)\n'
)
# Statement form with OUT: CALL "DBL",21 OUT V
MAIN_STMT_OUT = (
    'LOAD "PRG1:Q",0\n'
    'USE 1\n'
    'CALL "DBL",21 OUT V\n'
    'SAVE"TXT:O",STR$(V)\n'
)
# Same-slot function form (no slot machinery) to isolate the parse question.
MAIN_FUNC_SAME_SLOT = (
    'V=CALL("DBL",21)\n'
    'SAVE"TXT:O",STR$(V)\n'
)
HELPER_DBL_SLOT0 = ""  # not used for same-slot; DBL defined inline below instead

# Same-slot function form with DBL defined in slot 0 itself.
MAIN_FUNC_INLINE = (
    'COMMON DEF DBL N\nRETURN N*2\nEND\n'
    'V=CALL("DBL",21)\n'
    'SAVE"TXT:O",STR$(V)\n'
)

CASES = [
    ("call_func_form_cross_slot",  HELPER_DBL, MAIN_FUNC_FORM),
    ("call_stmt_out_cross_slot",   HELPER_DBL, MAIN_STMT_OUT),
    ("call_func_form_inline",      "",         MAIN_FUNC_INLINE),
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
