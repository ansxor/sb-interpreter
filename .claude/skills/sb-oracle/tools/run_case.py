#!/usr/bin/env python3
"""Run ONE SmileBASIC test on the real 3.6.0 oracle and capture its result from disk.

Full flow (all verified):
  raise Azahar -> clear DIRECT-mode line (SHIFT+BACKSPACE) -> type a command that SAVEs
  its result to a TXT file -> ENTER -> tap YES on the "Confirm - Write file" dialog ->
  poll for the fresh file on disk -> return its decoded contents (body[80:-20]).

Usage:
  run_case.py ready                           # launch Azahar if needed + probe (READY/NOT READY)
  run_case.py batch FILE [OUTFILE]            # harvest many `name|expr` lines in ONE process (recommended)
                                              #   OUTFILE -> incremental + resumable (survives a kill)
  run_case.py expr 'FLOOR(-2.1)'              # one case -> "-3" (numeric wraps in STR$)
  run_case.py expr 'MID$("ABCDE",1,2)' str    # one case, string result (no STR$ wrap)
  run_case.py prog 'FLOOR(8.9)'               # one case via the efficient program-file path
Run `ready` FIRST so cold-start/not-ready doesn't make each case eat a timeout. SB should be
on the DIRECT-mode screen (see SKILL.md Step 0).
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


def ready(tries=3):
    """Confirm the oracle is usable (Azahar up + SB on a command-running screen) by harvesting
    a trivial value. Launches Azahar if needed (raise_window). Returns True/False. Call this
    ONCE before a harvest so cases don't each eat a timeout on a cold/not-ready emulator."""
    for _ in range(tries):
        try:
            if run_expr("1+1") == "2":
                return True
        except Exception:  # noqa: BLE001
            time.sleep(2.0)
    return False


def _load_done(outpath):
    """Names already harvested OK in a prior (possibly killed) run — for resume. ERROR rows
    are NOT counted as done, so a re-run retries them."""
    done = {}
    try:
        for line in open(outpath):
            if "\t" not in line:
                continue
            name, _, val = line.rstrip("\n").partition("\t")
            if val and not val.startswith("ERROR"):
                done[name.strip()] = val
    except FileNotFoundError:
        pass
    return done


def batch(path, outpath=None):
    """Harvest many cases in ONE process (no backgrounding, no sleep-polling). Input file:
    one case per line, `name|expr` (or just `expr`); `#` comments allowed. Prints
    `name<TAB>result` (or `name<TAB>ERROR ...`) to stdout.

    Give an OUTFILE to make harvest INCREMENTAL + RESUMABLE: each result is appended (and
    flushed) the instant it lands, so a run killed mid-batch (timeout, out-of-credits) keeps
    everything harvested so far; re-running skips names already present with an OK value and
    retries only the failures. This is the recommended harvest path — point OUTFILE at a file
    the spec pass reads."""
    cases = []
    for line in open(path):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name, expr = line.split("|", 1) if "|" in line else (line, line)
        cases.append((name.strip(), expr.strip()))
    done = _load_done(outpath) if outpath else {}
    if done:
        print(f"# resume: {len(done)} case(s) already harvested in {outpath}, skipping them",
              flush=True)
    out = open(outpath, "a") if outpath else None
    if not ready():
        sys.exit("ORACLE NOT READY — launch Azahar and put SmileBASIC on the DIRECT-mode "
                 "screen (Step 0 in SKILL.md), then retry.")
    for name, expr in cases:
        if name in done:
            print(f"{name}\t{done[name]}\t(cached)", flush=True)
            continue
        try:
            row = f"{name}\t{run_expr(expr)}"
        except Exception as e:  # noqa: BLE001
            row = f"{name}\tERROR {e}"
        print(row, flush=True)
        if out:
            out.write(row + "\n")
            out.flush()
    if out:
        out.close()


if __name__ == "__main__":
    a = sys.argv[1:]
    if not a:
        print(__doc__)
        sys.exit(2)
    mode = a[0]
    if mode == "ready":                 # probe: is the oracle usable right now?
        print("READY" if ready() else "NOT READY")
    elif mode == "batch":               # recommended: harvest many cases from a file
        batch(a[1], a[2] if len(a) > 2 else None)
    elif mode == "expr":                # one case, typed: SAVE"TXT:O",STR$(<expr>)
        print(run_expr(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "prog":                # one case, efficient: write program to disk, LOAD+RUN
        print(run_expr_prog(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "progsrc":             # a full program (must SAVE its result)
        print(run_program(a[1]))
    else:                               # a verbatim DIRECT-mode command (must SAVE its result)
        print(run_command(a[1]))
