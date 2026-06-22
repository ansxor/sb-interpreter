#!/usr/bin/env python3
"""Run ONE SmileBASIC test on the real 3.6.0 oracle and capture its result from disk.

Full flow (all verified):
  raise Azahar -> clear DIRECT-mode line (SHIFT+BACKSPACE) -> type a command that SAVEs
  its result to a TXT file -> ENTER -> tap YES on the "Confirm - Write file" dialog ->
  poll for the fresh file on disk -> return its decoded contents (body[80:-20]).

Usage:
  run_case.py expr 'FLOOR(-2.1)'              # wraps as SAVE"TXT:O",STR$(FLOOR(-2.1)) -> "-3"
  run_case.py expr 'MID$("ABCDE",1,2)' str    # string result (no STR$ wrap)
  run_case.py cmd  'SAVE"TXT:O",STR$(1+1)'    # run a full command verbatim
"""
import sys
import time

import sb_extdata as X
import sb_window as W


def run_command(command, result_name="O", ftype="TXT", attempts=5):
    W.raise_window()
    time.sleep(0.4)
    # Dismiss any stale dialog a prior run may have left open, so typing lands cleanly.
    W.press("YES")
    time.sleep(0.5)
    before = X.result_mtime(result_name, ftype)
    W.clear_line()
    time.sleep(0.3)
    W.type_str(command)
    time.sleep(0.3)
    W.enter()
    # The "Write file" / "overwrite?" dialog renders within ~1s; confirm it in bounded
    # rounds (slow enough to avoid stray taps, few enough to avoid junk).
    last = None
    for _ in range(attempts):
        time.sleep(1.2)
        W.press("YES")
        time.sleep(0.6)
        try:
            mt = X.result_mtime(result_name, ftype)
            if mt is not None and mt != before:
                return X.read_result(result_name, ftype)
        except Exception as e:  # noqa: BLE001
            last = e
    raise TimeoutError(f"no fresh result {result_name!r} after {attempts} confirm attempts (last: {last})")


def run_expr(expr, result_name="O", numeric=True):
    """Evaluate a single SB expression and capture its value. numeric -> wrap in STR$."""
    inner = f"STR$({expr})" if numeric else expr
    cmd = f'SAVE"TXT:{result_name}",{inner}'
    return run_command(cmd, result_name)


def run_program(source, result_name="O", prog="P", attempts=6):
    """EFFICIENT path: write `source` to extdata as a program, then type only the fixed
    short commands `LOAD"PRG0:<prog>",0` (the ,0 auto-dismisses the load dialog) and `RUN`.
    The program must SAVE its result to TXT:<result_name>. Avoids typing the whole program.
    """
    # Programs are TXT files; LOAD"PRG0:<prog>" reads on-disk "T"+<prog>. write_file emits
    # a valid file (correct header + HMAC footer) so SB accepts it.
    X.write_file(prog, source if source.endswith("\n") else source + "\n", "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.press("YES")  # clear any stale dialog
    time.sleep(0.5)
    before = X.result_mtime(result_name, "TXT")
    W.clear_line()
    time.sleep(0.3)
    W.type_str(f'LOAD"PRG0:{prog}",0')
    time.sleep(0.2)
    W.enter()
    time.sleep(1.2)  # line is empty after LOAD+ENTER; no clear needed
    W.type_str("RUN")
    time.sleep(0.2)
    W.enter()
    last = None
    for _ in range(attempts):
        time.sleep(1.2)
        W.press("YES")  # confirm the program's SAVE dialog
        time.sleep(0.6)
        try:
            mt = X.result_mtime(result_name, "TXT")
            if mt is not None and mt != before:
                return X.read_result(result_name, "TXT")
        except Exception as e:  # noqa: BLE001
            last = e
    raise TimeoutError(f"no fresh result {result_name!r} (last: {last})")


def run_expr_prog(expr, result_name="O", numeric=True):
    """Like run_expr but via the efficient program-file path."""
    inner = f"STR$({expr})" if numeric else expr
    return run_program(f'SAVE"TXT:{result_name}",{inner}', result_name)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    mode, arg = sys.argv[1], sys.argv[2]
    numeric = not (len(sys.argv) > 3 and sys.argv[3] == "str")
    if mode == "expr":          # typed: SAVE"TXT:O",STR$(<expr>)
        print(run_expr(arg, numeric=numeric))
    elif mode == "prog":        # efficient: write <expr> program to disk, LOAD+RUN
        print(run_expr_prog(arg, numeric=numeric))
    elif mode == "progsrc":     # efficient: write a full program (must SAVE its result)
        print(run_program(arg))
    else:                       # run a verbatim DIRECT-mode command (must SAVE its result)
        print(run_command(arg))
