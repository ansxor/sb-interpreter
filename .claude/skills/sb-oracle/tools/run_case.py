#!/usr/bin/env python3
"""Run ONE SmileBASIC test on the real 3.6.0 oracle and capture its result from disk.

Mechanism (all verified). The SMILE button runs OBOOT (smile_boot.sb), which arms five
function keys, so every per-case action is a single calibrated KEY tap — never the old
char-by-char typing (the flakiness source):

    F1 = LOAD"PRG0:P",0     F4 = RUN                  (load, then run program P)
    F2 = SAVE err -> O      F3 = SAVE "__OK__" -> O    F5 = CLS

A SmileBASIC SAVE is a TWO-dialog sequence (Confirm -> Yes, then Information -> OK); LOAD
from a key slot adds a one-tap load dialog (the ,0 only auto-dismisses when TYPED, not from a
key). `W.confirm_dialogs()` closes whatever is open by tapping the YES/OK button until the
on-screen dialog is gone (verified by sampling the screen) — so every save self-closes and no
stale dialog is ever left for the next op. Each run DELETES result file O first, so "O exists"
unambiguously means "fresh result from this run" (kills the old stale-read / errline=0 ghost).

Usage:
  run_case.py ready                       # tap SMILE: arm keys + write '__OK__' to O + verify
  run_case.py batch FILE [OUTFILE]        # FAST harvest: ONE mega-program for value cases
                                          #   lines: `name|expr` / `name|expr|str` /
                                          #          `name|stmt|err` (expects a raise) / bare `expr`
                                          #   OUTFILE -> incremental + resumable (survives a kill)
  run_case.py prog 'FLOOR(8.9)'           # one value case via the program path -> "8"
  run_case.py prog 'MID$("ABCDE",1,2)' str   # one case, string result (no STR$ wrap)
  run_case.py errcase 'A=SQR(-1)'         # one error case -> {errored, errnum, errline}
  run_case.py grp draw.sb out.png [crop] [page]  # graphics golden: draw -> SAVE GRPn -> PNG
                                          #   crop: full(512²,default) | top(400×240) | bottom | WxH
  run_case.py screenshot out.png          # composite golden (sprites/BG/both screens): Ctrl+P
Run `ready` FIRST — it taps SMILE to (re)arm the keys and proves the SAVE->dialog->disk path,
so cases don't each eat a timeout. SB must be on the DIRECT-mode screen (see SKILL.md Step 0).
`batch` writes ONE program that SAVEs all VALUE results at once (≈one LOAD+RUN, not one per
case) and bisects around any case that halts. `err` cases can't batch (each halts the program)
so each runs alone and its ERRNUM/ERRLINE are read via F2 after the halt.
"""
import os
import re
import sys
import time

import sb_extdata as X
import sb_grp as G
import sb_window as W

RUN_SETTLE = 1.2   # seconds to let a LOAD/RUN render its dialog before confirming it


# ── result-file helpers (delete-first => "O exists" means a fresh result) ─────────────────
def _result_path(name="O", ftype="TXT"):
    return X._path(X.TYPE_PREFIX[ftype] + name)


def _delete_result(name="O", ftype="TXT"):
    p = _result_path(name, ftype)
    if os.path.exists(p):
        os.remove(p)


def _read_result(name="O", ftype="TXT"):
    return X.read_result(name, ftype) if os.path.exists(_result_path(name, ftype)) else None


# ── per-case key actions (each closes the dialog(s) it raises) ─────────────────────────────
def _clean():
    """F5 = CLS — clear the console between runs (no dialog raised)."""
    W.press("F5")
    time.sleep(0.6)


def _load_prog():
    """F1 = LOAD"PRG0:P",0, then clear the load-confirm dialog. (The ,0 auto-dismisses only
    when typed, not from a key slot — so a single YES is needed; confirm_dialogs handles it.)"""
    W.press("F1")
    time.sleep(RUN_SETTLE)
    W.confirm_dialogs()


def _run_prog():
    """F4 = RUN, then clear the program's SAVE dialogs (Confirm + Information). If the program
    halted on an error before its SAVE, no dialog is up and confirm_dialogs is a no-op."""
    W.press("F4")
    time.sleep(RUN_SETTLE)
    W.confirm_dialogs()


def run_program(source, result_name="O", ftype="TXT"):
    """Write `source` to program slot P (it must SAVE its result to <result_name>), load+run it
    via F1/F4, and return the result file contents (or None if no file was produced)."""
    X.write_file("P", source if source.endswith("\n") else source + "\n", "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.confirm_dialogs()                 # clear any stale dialog (screen-verified no-op if none)
    _delete_result(result_name, ftype)
    _clean()
    _load_prog()
    _run_prog()
    return _read_result(result_name, ftype)


def run_expr_prog(expr, result_name="O", numeric=True):
    """One value case via the program path: P = `SAVE"TXT:O",STR$(<expr>)` (or unwrapped for
    a string result)."""
    inner = f"STR$({expr})" if numeric else f"({expr})"
    return run_program(f'SAVE"TXT:{result_name}",{inner}', result_name)


def ready(tries=3):
    """Tap SMILE -> runs OBOOT, which arms KEY 1-5 and SAVEs '__OK__' to O. Close its dialogs
    and check O=='__OK__'. This both (re)arms the keys and proves the SAVE->dialog->disk path
    the whole harvest depends on — no screenshot/OCR. Returns True/False. Call ONCE up front.
    (Requires OBOOT assigned to the SMILE button in SB's settings — a one-time manual step.)"""
    W.raise_window()
    time.sleep(0.4)
    for _ in range(tries):
        W.confirm_dialogs()             # clear any stale dialog first
        _delete_result()
        W.press("SMILE")
        time.sleep(1.0)
        W.confirm_dialogs()
        if (_read_result() or "").strip() == "__OK__":
            return True
        time.sleep(1.5)
    return False


def setup_keys():
    """Arm the function keys by tapping SMILE (runs OBOOT). Same as ready()'s arm step, kept as
    a separate verb. Prefer `ready`, which also verifies."""
    W.raise_window()
    time.sleep(0.4)
    W.confirm_dialogs()
    _delete_result()
    W.press("SMILE")
    time.sleep(1.0)
    W.confirm_dialogs()
    return "tapped SMILE (OBOOT arms KEY 1-5)"


def run_command(command, result_name="O", ftype="TXT"):
    """LEGACY typed path: type a verbatim DIRECT-mode command that SAVEs its result, char by
    char. Prefer the key-based program path (run_program); this stays for ad-hoc one-offs."""
    W.raise_window()
    time.sleep(0.4)
    W.confirm_dialogs()
    _delete_result(result_name, ftype)
    W.clear_line()
    time.sleep(0.3)
    W.type_str(command)
    time.sleep(0.3)
    W.enter()
    time.sleep(RUN_SETTLE)
    W.confirm_dialogs()
    return _read_result(result_name, ftype)


def run_expr(expr, result_name="O", numeric=True):
    """LEGACY typed path: evaluate one expression by typing SAVE"TXT:O",STR$(<expr>)."""
    inner = f"STR$({expr})" if numeric else expr
    return run_command(f'SAVE"TXT:{result_name}",{inner}', result_name)


# ── Fast harvest: one mega-program for many value cases ───────────────────────────────────
#
# Typing each case was the old slow + flaky part. Now value cases are batched into ONE program
# that evaluates them all into a string and SAVEs once — a 60-case harvest is one LOAD+RUN+read.
# SB has NO error trapping, so a case that raises halts the program before the SAVE and we get
# no file; `harvest` bisects the batch to isolate the offender and still collect every other case.

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


def run_error_case(stmt, result_name="O"):
    """Capture the error of a statement EXPECTED to raise. The program is `<stmt>` then a
    sentinel SAVE; if <stmt> raises, SB halts before the sentinel (no error trapping) so result
    file O never appears — we then tap F2 to SAVE ERRNUM/ERRLINE (set by the halt, readable in
    DIRECT mode). `stmt` must be a STATEMENT (e.g. `A=SQR(-1)`). Returns
    {"errored": bool, "errnum": int|None, "errline": int|None}."""
    X.write_file("P", f'{stmt}\nSAVE"TXT:{result_name}","__OK__"\n', "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.confirm_dialogs()
    _delete_result(result_name)
    _clean()
    _load_prog()
    _run_prog()
    val = _read_result(result_name)
    if val is not None and val.strip() == "__OK__":
        return {"errored": False, "errnum": None, "errline": None}
    # Halted before the sentinel: read ERRNUM/ERRLINE via F2 (the error halt set them).
    _delete_result(result_name)
    W.press("F2")
    time.sleep(RUN_SETTLE)
    W.confirm_dialogs()
    val = _read_result(result_name)
    if val is not None:
        f = val.strip().split("\t")
        num = int(f[0]) if f and f[0].lstrip("-").isdigit() else None
        line = int(f[1]) if len(f) > 1 and f[1].lstrip("-").isdigit() else None
        return {"errored": True, "errnum": num, "errline": line}
    return {"errored": True, "errnum": None, "errline": None}


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


def _run_batch_program(cases, result_name="O"):
    """Write+run the mega-program for `cases` via the key path; return the result file text, or
    None if the program halted (no file — a case raised a runtime error)."""
    return run_program(_build_batch_program(cases, result_name), result_name)


def harvest(cases, result_name="O", on_result=None):
    """Harvest values for many (name, expr, mode) value cases via the mega-program, bisecting
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


def capture_grp(program, out_png=None, page=0, name="G", crop=None):
    """GRAPHICS GOLDEN: run a drawing program, SAVE graphics page <page> to disk, decode the
    GRP to RGBA, and (optionally) write a PNG. `program` is SB source that draws to the page;
    we append the `SAVE"GRP<page>:<name>"` for you. `crop=(w,h)` writes only the top-left
    region (e.g. (400,240) top screen) instead of the full 512x512 page. Returns (w, h, rgba).

    GRP pages (GRP0-5) are 512x512 buffers INDEPENDENT of XSCREEN/display mode — this reads the
    page buffer off disk, not a screenshot, so the mode can't corrupt it. For content on both
    screens, capture each page in use (`page=`). For the composited display (sprites+BG, XSCREEN
    4 combined, or 3D), use capture_screen instead."""
    src = program.rstrip("\n") + f'\nSAVE"GRP{page}:{name}"\n'
    X.write_file("P", src, "TXT")
    W.raise_window()
    time.sleep(0.4)
    W.confirm_dialogs()
    _delete_result(name, "GRP")
    _clean()
    _load_prog()
    _run_prog()
    if not os.path.exists(_result_path(name, "GRP")):
        raise TimeoutError("no fresh GRP file — did the program draw + did SAVE's dialog confirm?")
    w, h, rgba = G.decode_grp(X.read_raw(X.TYPE_PREFIX["GRP"] + name))
    if crop:
        rgba = G.crop(w, h, rgba, crop[0], crop[1])
        w, h = crop
    if out_png:
        G.write_png(out_png, w, h, rgba)
    return w, h, rgba


def capture_screen(out_png="/tmp/sb_screen.png"):
    """COMPOSITE GOLDEN (sprites/BG/console): Azahar's Ctrl+P screenshot of the rendered screen.
    Unlike capture_grp (the exact GRP page), this is the composited display (all layers) at the
    emulator's output resolution — use it for sprite/BG goldens that GSAVE can't reach. Returns
    the newest PNG path in Azahar's screenshot dir (copied to out_png)."""
    import glob
    import shutil
    shotdir = os.path.expanduser("~/Library/Application Support/Azahar/screenshots")
    W.raise_window()
    time.sleep(0.5)
    pre = set(glob.glob(os.path.join(shotdir, "*.png")))
    W.key_combo("ctrl", "p")                # Capture Screenshot shortcut
    for _ in range(10):
        time.sleep(0.6)
        new = set(glob.glob(os.path.join(shotdir, "*.png"))) - pre
        if new:
            src = max(new, key=os.path.getmtime)
            shutil.copy(src, out_png)
            return out_png
    raise TimeoutError("no new screenshot appeared (is Ctrl+P bound + window focused?)")


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
    line, `name|expr` (or `name|expr|str`, `name|stmt|err`, or bare `expr`); `#` comments OK.
    Prints `name<TAB>result` (or `name<TAB>ERROR ...`) to stdout.

    Give an OUTFILE to make harvest INCREMENTAL + RESUMABLE: each result is appended (and
    flushed) the instant it resolves, so a killed run keeps everything so far; re-running skips
    names already present with an OK value and retries only the failures."""
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
        sys.exit("ORACLE NOT READY — launch Azahar, put SmileBASIC on the DIRECT-mode screen, "
                 "and make sure OBOOT is assigned to the SMILE button (Step 0 in SKILL.md).")
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
    if mode == "ready":                 # tap SMILE: arm keys + write '__OK__' + verify
        print("READY" if ready() else "NOT READY")
    elif mode == "setupkeys":           # tap SMILE to arm KEY 1-5 (no verify)
        print(setup_keys())
    elif mode == "batch":               # recommended: FAST harvest via one mega-program
        batch(a[1], a[2] if len(a) > 2 else None)
    elif mode == "prog":                # one value case via the key/program path
        print(run_expr_prog(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "expr":                # one value case via the LEGACY typed path
        print(run_expr(a[1], numeric=not (len(a) > 2 and a[2] == "str")))
    elif mode == "errcase":             # one error case: run a halting statement -> ERRNUM/ERRLINE
        print(run_error_case(a[1]))
    elif mode == "grp":                 # graphics golden: draw (PROGFILE) -> SAVE GRP -> decode -> PNG
        # grp PROGFILE OUT.png [crop] [page]
        src = open(a[1]).read()
        out = a[2] if len(a) > 2 else None
        crops = {"full": None, "top": (400, 240), "bottom": (320, 240)}
        carg = a[3] if len(a) > 3 else "full"
        crop = crops[carg] if carg in crops else (
            tuple(int(x) for x in carg.split("x")) if "x" in carg else None)
        page = int(a[4]) if len(a) > 4 else 0
        w, h, _ = capture_grp(src, out, page=page, crop=crop)
        print(f"captured GRP{page} {w}x{h} -> {out or '(no png)'}")
    elif mode == "screenshot":          # composite golden: Azahar Ctrl+P screenshot -> PNG
        print("saved:", capture_screen(a[1] if len(a) > 1 else "/tmp/sb_screen.png"))
    elif mode == "progsrc":             # a full program (must SAVE its result)
        print(run_program(a[1]))
    else:                               # a verbatim DIRECT-mode command (legacy typed path)
        print(run_command(a[1]))
