#!/usr/bin/env python3
"""Run ONE SmileBASIC test on the real 3.6.0 oracle and capture its result from disk.

Full flow (all verified):
  raise Azahar -> clear DIRECT-mode line (SHIFT+BACKSPACE) -> type a command that SAVEs
  its result to a TXT file -> ENTER -> tap YES on the "Confirm - Write file" dialog ->
  poll for the fresh file on disk -> return its decoded contents (body[80:-20]).

Usage:
  run_case.py ready                           # launch Azahar if needed + probe (READY/NOT READY)
  run_case.py setupkeys                        # assign F1 = LOAD+RUN macro (one tap runs a program)
  run_case.py batch FILE [OUTFILE]            # FAST harvest: ONE mega-program for value cases
                                              #   lines: `name|expr` / `name|expr|str` /
                                              #          `name|stmt|err` (expects a raise) / bare `expr`
                                              #   OUTFILE -> incremental + resumable (survives a kill)
  run_case.py expr 'FLOOR(-2.1)'              # one case -> "-3" (numeric wraps in STR$)
  run_case.py expr 'MID$("ABCDE",1,2)' str    # one case, string result (no STR$ wrap)
  run_case.py prog 'FLOOR(8.9)'               # one case via the efficient program-file path
  run_case.py errcase 'A=SQR(-1)'             # one error case -> {errored, errnum, errline}
Run `ready` FIRST so cold-start/not-ready doesn't make each case eat a timeout. SB should be
on the DIRECT-mode screen (see SKILL.md Step 0). `batch` writes ONE program that SAVEs all
VALUE results at once (≈one LOAD+RUN, not one per case) and bisects around any case that halts.
SB has no error trapping, so `err` cases can't batch — each runs alone, and ERRNUM/ERRLINE are
read in DIRECT mode after the halt.
"""
import re
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


# ── Fast harvest: one mega-program for many cases + a one-tap KEY run macro ──────────────
#
# Typing each `SAVE"TXT:O",STR$(<expr>)` into DIRECT mode is the slow part (~tens of seconds
# per case: dozens of on-screen key taps + a confirm dialog). Instead we write ONE program
# that evaluates ALL cases into a single string and SAVEs it once — so a 60-case harvest is
# ONE LOAD+RUN+read instead of 60. SmileBASIC has NO error trapping, so if a case raises a
# runtime error the program halts before the SAVE and we get no file; `harvest` bisects the
# batch to isolate the offender and still collect every other case.

def setup_keys(prog="P"):
    """Assign function key F1 = `LOAD"PRG0:<prog>",0:RUN` + CHR$(13) (the trailing CR auto-
    runs it). Run ONCE per session, after `ready`. Then every program runs in a SINGLE F1 tap
    — the user's "reset & run" macro. NOTE: pressing F1 needs its tap coordinate in
    keymap.json under "F1"; calibrate it once (`sb_window.py calibrate X Y` + screenshot).
    Until then the run trigger falls back to typing LOAD+RUN (fine — it fires ~once/batch)."""
    W.raise_window()
    time.sleep(0.4)
    W.press("YES")
    time.sleep(0.4)
    W.clear_line()
    time.sleep(0.3)
    macro = f'KEY 1,"LOAD"+CHR$(34)+"PRG0:{prog}"+CHR$(34)+",0:RUN"+CHR$(13)'
    W.type_str(macro)
    time.sleep(0.2)
    W.enter()
    return macro


def _trigger_run(prog="P"):
    """Load+run PRG0:<prog>. One F1 tap if the KEY macro is calibrated (setup_keys), else type
    the LOAD+RUN (with the mega-program this fires ~once per batch, so typing it is cheap)."""
    if "F1" in W.load_keymap():
        W.press("F1")                       # KEY 1 macro: LOAD"PRG0:P",0:RUN + CHR$(13)
        return
    W.type_str(f'LOAD"PRG0:{prog}",0')
    time.sleep(0.2)
    W.enter()
    time.sleep(1.2)                         # line is empty after LOAD+ENTER; no clear needed
    W.type_str("RUN")
    time.sleep(0.2)
    W.enter()


_MODE_TAGS = {"str": "str", "s": "str", "string": "str", "num": "num", "n": "num",
              "number": "num", "err": "err", "error": "err", "e": "err"}


def _parse_case_line(line):
    """`name|expr`, `name|expr|<mode>`, or bare `expr`. Mode tag picks capture: `num` (default,
    STR$-wrapped), `str` (string result, no wrap), or `err` (statement EXPECTED to raise —
    capture ERRNUM/ERRLINE). Returns (name, expr, mode)."""
    parts = line.split("|")
    name = parts[0].strip()
    if not re.match(r"^[\w.\-]+$", name):
        raise ValueError(f"case name {name!r} must be identifier-like (letters/digits/_.-)")
    if len(parts) == 1:
        return (name, name, "num")
    tag = parts[-1].strip().lower()
    if tag in _MODE_TAGS:
        return (name, "|".join(parts[1:-1]).strip(), _MODE_TAGS[tag])
    return (name, "|".join(parts[1:]).strip(), "num")


def _build_batch_program(cases, result_name="O"):
    """One SB program: evaluate every value case into R$ as `name<TAB>value` lines (LF-
    separated), SAVE once. `cases` = list of (name, expr, mode); `err` cases are NOT included
    (they halt the program — capture them with run_error_case)."""
    lines = ['R$=""']
    for name, expr, mode in cases:
        val = f"({expr})" if mode == "str" else f"STR$({expr})"
        lines.append(f'R$=R$+"{name}"+CHR$(9)+{val}+CHR$(10)')
    lines.append(f'SAVE"TXT:{result_name}",R$')
    return "\n".join(lines)


def run_error_case(stmt, result_name="O", prog="P", sentinel_attempts=3):
    """Capture the error of a statement EXPECTED to raise. SB has no error trapping, so the
    statement halts the program; afterwards ERRNUM/ERRLINE hold that error and are readable in
    DIRECT mode. Flow: run `<stmt>` followed by a sentinel SAVE — if the sentinel file appears
    the statement did NOT raise; otherwise it halted, and we read ERRNUM/ERRLINE via a DIRECT
    save. `stmt` must be a STATEMENT (e.g. `A=SQR(-1)`), not a bare expression. Returns
    {"errored": bool, "errnum": int|None, "errline": int|None}."""
    X.write_file(prog, f'{stmt}\nSAVE"TXT:{result_name}","__OK__"\n', "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.press("YES")
    time.sleep(0.5)
    before = X.result_mtime(result_name, "TXT")
    W.clear_line()
    time.sleep(0.3)
    _trigger_run(prog)
    for _ in range(sentinel_attempts):
        time.sleep(1.2)
        W.press("YES")
        time.sleep(0.6)
        mt = X.result_mtime(result_name, "TXT")
        if mt is not None and mt != before:
            if X.read_result(result_name, "TXT").strip() == "__OK__":
                return {"errored": False, "errnum": None, "errline": None}
            break  # a file appeared but it isn't the sentinel — treat as halted
    # Halted on the error: read ERRNUM/ERRLINE (set by THIS run's error) in DIRECT mode.
    before2 = X.result_mtime(result_name, "TXT")
    W.clear_line()
    time.sleep(0.3)
    W.type_str(f'SAVE"TXT:{result_name}",STR$(ERRNUM)+CHR$(9)+STR$(ERRLINE)')
    time.sleep(0.2)
    W.enter()
    for _ in range(5):
        time.sleep(1.2)
        W.press("YES")
        time.sleep(0.6)
        mt = X.result_mtime(result_name, "TXT")
        if mt is not None and mt != before2:
            f = X.read_result(result_name, "TXT").strip().split("\t")
            num = int(f[0]) if f and f[0].lstrip("-").isdigit() else None
            line = int(f[1]) if len(f) > 1 and f[1].lstrip("-").isdigit() else None
            return {"errored": True, "errnum": num, "errline": line}
    return {"errored": True, "errnum": None, "errline": None}  # halted but couldn't read


def _parse_batch_result(text):
    """Parse `name<TAB>value` lines back into {name: value}. A line with no TAB is a
    continuation of the previous value (values may contain newlines)."""
    recs, cur = {}, None
    for raw in text.split("\n"):
        line = raw.rstrip("\r")
        if "\t" in line:
            name, _, val = line.partition("\t")
            recs[name], cur = val, name
        elif cur is not None and line:
            recs[cur] += "\n" + line
    return recs


def _run_batch_program(cases, result_name="O", attempts=7):
    """Write+run the mega-program for `cases`; return the result file text, or None if the
    program halted (no fresh file within the confirm window — a case errored)."""
    X.write_file("P", _build_batch_program(cases, result_name), "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.press("YES")                          # clear any stale dialog
    time.sleep(0.5)
    before = X.result_mtime(result_name, "TXT")
    W.clear_line()
    time.sleep(0.3)
    _trigger_run("P")
    for _ in range(attempts):
        time.sleep(1.2)
        W.press("YES")                      # confirm the program's SAVE dialog
        time.sleep(0.6)
        try:
            mt = X.result_mtime(result_name, "TXT")
            if mt is not None and mt != before:
                return X.read_result(result_name, "TXT")
        except Exception:                   # noqa: BLE001
            pass
    return None


def harvest(cases, result_name="O", on_result=None):
    """Harvest values for many (name, expr, numeric) cases via the mega-program, bisecting
    around any case that halts the program (SB has no error trapping). Returns {name: value
    or 'ERROR ...'}. `on_result(name, value)` is called as each case resolves (for streaming
    + resumable persistence)."""
    results = {}

    def emit(n, v):
        results[n] = v
        if on_result:
            on_result(n, v)

    def rec(group):
        if not group:
            return
        text = _run_batch_program(group, result_name)
        if text is not None:
            recs = _parse_batch_result(text)
            if all(n in recs for n, _, _ in group):
                for n, _, _ in group:
                    emit(n, recs[n])
                return
        # No (complete) file: one case in this group raised a runtime error and halted.
        if len(group) == 1:
            emit(group[0][0], "ERROR halted (no result — runtime error?)")
            return
        mid = len(group) // 2
        rec(group[:mid])
        rec(group[mid:])

    rec(cases)
    return results


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
    """Harvest many cases FAST via one mega-program (see `harvest`). Input file: one case per
    line, `name|expr` (or `name|expr|str`, or bare `expr`); `#` comments allowed. Prints
    `name<TAB>result` (or `name<TAB>ERROR ...`) to stdout.

    Give an OUTFILE to make harvest INCREMENTAL + RESUMABLE: each result is appended (and
    flushed) the instant it resolves, so a run that's killed keeps everything so far; re-running
    skips names already present with an OK value and retries only the failures. This is the
    recommended harvest path — point OUTFILE at a file the spec pass reads."""
    cases = [_parse_case_line(line.strip()) for line in open(path)
             if line.strip() and not line.strip().startswith("#")]
    done = _load_done(outpath) if outpath else {}
    if done:
        print(f"# resume: {len(done)} case(s) already harvested in {outpath}, skipping them",
              flush=True)
    for name, _, _ in cases:
        if name in done:
            print(f"{name}\t{done[name]}\t(cached)", flush=True)
    remaining = [c for c in cases if c[0] not in done]
    if not remaining:
        return
    if not ready():
        sys.exit("ORACLE NOT READY — launch Azahar and put SmileBASIC on the DIRECT-mode "
                 "screen (Step 0 in SKILL.md), then retry.")
    out = open(outpath, "a") if outpath else None

    def on_result(name, value):
        print(f"{name}\t{value}", flush=True)
        if out:
            out.write(f"{name}\t{value}\n")
            out.flush()

    # Value cases batch into one mega-program; `err` cases must each run alone (they halt).
    value_cases = [c for c in remaining if c[2] != "err"]
    error_cases = [c for c in remaining if c[2] == "err"]
    if value_cases:
        harvest(value_cases, on_result=on_result)
    for name, stmt, _ in error_cases:
        r = run_error_case(stmt)
        if not r["errored"]:
            on_result(name, "NOERR (statement did not raise)")
        elif r["errnum"] is not None:
            el = f" errline={r['errline']}" if r["errline"] is not None else ""
            on_result(name, f"errnum={r['errnum']}{el}")
        else:
            on_result(name, "ERROR errnum capture failed (halted but no read)")
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
    elif mode == "setupkeys":           # assign F1 = LOAD+RUN macro (one-tap runs, once/session)
        print("KEY 1 set:", setup_keys())
    elif mode == "batch":               # recommended: FAST harvest via one mega-program
        batch(a[1], a[2] if len(a) > 2 else None)
    elif mode == "expr":                # one case, typed: SAVE"TXT:O",STR$(<expr>)
        print(run_expr(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "prog":                # one case, efficient: write program to disk, LOAD+RUN
        print(run_expr_prog(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "errcase":             # one error case: run a halting statement -> ERRNUM/ERRLINE
        print(run_error_case(a[1]))
    elif mode == "progsrc":             # a full program (must SAVE its result)
        print(run_program(a[1]))
    else:                               # a verbatim DIRECT-mode command (must SAVE its result)
        print(run_command(a[1]))
