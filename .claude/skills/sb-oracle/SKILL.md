---
name: sb-oracle
description: >
  Drive real SmileBASIC 3.6.0 in the Azahar emulator on macOS to capture ground-truth
  behavior (the "oracle"). Programs are injected as extdata files (valid header + HMAC-SHA1
  footer) and run via cliclick touch; results are read from extdata files on disk. Azahar
  has NO InputRedirection and the RPC crashes SB on heavy use, so neither is used. Use when
  harvesting hw_verified spec data.
metadata:
  tags: [smilebasic, azahar, oracle, cliclick, extdata, macos]
---

# sb-oracle — drive real SmileBASIC 3.6.0 in Azahar

Capture ground truth from real SB 3.6.0 to fill `hw_verified` specs (see `prd/oracle.md`).
**Both paths work and are verified.**

## Quick start

```bash
cd .claude/skills/sb-oracle/tools
python3 run_case.py ready                          # STEP 0: launch Azahar (if needed) + probe -> READY
python3 run_case.py setupkeys                       # OPTIONAL: assign F1 = LOAD+RUN macro (one-tap runs)
python3 run_case.py batch cases.txt out.tsv        # RECOMMENDED: FAST harvest (one mega-program)
python3 run_case.py prog 'FLOOR(-2.1)'             # one case via the program-file path -> -3
python3 run_case.py expr 'MID$("ABCDE",2,3)' str   # one typed case, string -> BCD
python3 run_case.py errcase 'A=SQR(-1)'            # error case -> {errored, errnum, errline}
```
Run `ready` FIRST — it cold-starts Azahar and confirms SB is usable, so cases don't each eat a
timeout (a `sb_window.py bounds` that returns coords is NOT proof of readiness).

**`batch` is fast** — instead of typing each case into DIRECT mode (the slow part: dozens of
on-screen taps + a confirm dialog *per case*), it writes ONE program that evaluates ALL value
cases into a single string and `SAVE`s it once, then does a single LOAD+RUN+read. A 60-case
slice is ~one run, not 60. SmileBASIC has **no error trapping**, so if a value case unexpectedly
raises, the program halts before the SAVE; `batch` then **bisects** the group to isolate the
offender (marked `ERROR`) and still collects every other case. Lines are `name|expr`,
`name|expr|str` (string result, no `STR$` wrap), `name|stmt|err` (error case, below), or bare `expr`.

**Error cases (`errnum`/`errline`) — O-T5.** No error trapping means an error HALTS the program
and there is **no way to resume or catch it** (even `EXEC`/`RUN n` into another slot can't return
after an error — so the multi-slot idea doesn't help). But after the halt, `ERRNUM`/`ERRLINE`
hold that error and are readable in DIRECT mode. So an `err` case runs ALONE: the program is
`<stmt>` + a sentinel `SAVE"TXT:O","__OK__"`; if the sentinel file appears the statement didn't
raise (`NOERR`), otherwise it halted and we read `ERRNUM`/`ERRLINE` via a DIRECT save. Write the
case as a **statement** (`A=SQR(-1)`), not a bare expression. These can't batch (one run each),
but errors are a minority of cases.

**Always pass an OUTFILE** (each `name<TAB>result` is appended + flushed as it resolves): a run
that's killed keeps everything so far — re-running with the same OUTFILE skips OK rows and
retries only `ERROR` ones. Harvest a slice, fold it into the spec, harvest the next.

Verified: FLOOR(3.7)=3, FLOOR(-2.1)=-3, FLOOR(5)=5, FLOOR(8.9)=8, 7 DIV 2=3, 7 MOD 3=1,
LEN("ABCDE")=5, ABS(-9)=9, POW(2,10)=1024, SQR(144)=12.

## Setup (once per session)
0. **Launch first:** `run_case.py ready` opens Azahar if needed and waits for boot. SB must end
   up on the **DIRECT-mode screen** (keyboard visible); if it boots to a menu, navigate there
   once. `ready` (a real harvest probe) is the readiness signal — not `sb_window.py bounds`.
1. Screen Recording + Accessibility granted to the terminal app; `cliclick` installed.

## Mechanism
- **Raise window:** `open -a Azahar` (osascript `activate` is unreliable). Window pinned to
  pos(60,80) size(400,539) for stable coordinates.
- **Touch:** `cliclick` at screen `(60+wx, 80+wy)`. Keymap (`keymap.json`, from `inputs.json`
  via `gen_keymap.py`, transform `window = input + (40,272)`, scale 1.0) — verified accurate.
- **Type/execute:** `type_str` taps keys; `enter()` executes; `clear_line()` = SHIFT+BACKSPACE.
- **Efficient path:** `sb_extdata.write_file()` writes a valid program to extdata, then type
  `LOAD"PRG0:<name>",0` (the `,0` auto-dismisses the load dialog) + `RUN`. The program SAVEs
  its result; the **Write-file/overwrite dialog** is confirmed by a polled **YES** tap; the
  result file is read from disk.
- **Mega-program (the fast harvest):** `batch` writes ONE program — `R$=""` then a
  `R$=R$+"<name>"+CHR$(9)+STR$(<expr>)+CHR$(10)` line per case, then `SAVE"TXT:O",R$` — so all
  results come back in a single file (`name<TAB>value`, LF-separated). One LOAD+RUN for the whole
  slice. No in-program error handling exists in SB, so a halting case yields no file → `harvest`
  bisects to find it. (Only the program *source* is on disk; nothing long is typed.)
- **One-tap run macro (KEY/F1):** `setup_keys` assigns `KEY 1,"LOAD"+CHR$(34)+"PRG0:P"+CHR$(34)
  +",0:RUN"+CHR$(13)` — the trailing **`CHR$(13)`** (carriage return, per the `KEY` docs — *not*
  `CHR$(10)`) makes pressing **F1** load+run program `P` in a single tap (a "reset & run"). The
  run trigger uses F1 when `keymap.json` has an `"F1"` coord; otherwise it types `LOAD…+RUN`
  (fine — with the mega-program that's ~once per slice). **To enable one-tap: calibrate F1 once**
  (`sb_window.py calibrate <wx> <wy>` against a screenshot, add `"F1": [wx,wy]` to `keymap.json`).
- **Error capture (`run_error_case`):** run `<stmt>` + a `SAVE"TXT:O","__OK__"` sentinel; if the
  sentinel appears it didn't raise, else read `SAVE"TXT:O",STR$(ERRNUM)+CHR$(9)+STR$(ERRLINE)` in
  DIRECT mode (the error halted the program; `ERRNUM`/`ERRLINE` persist into DIRECT mode and were
  set by this run, so no stale-value risk). Freeze `errnum` into the spec test's `expect.error`.

## extdata file format (fully cracked, validated both directions)
Path: `~/Library/Application Support/Azahar/sdmc/Nintendo 3DS/<0*32>/<0*32>/extdata/00000000/000016DE/user/###/<DISKNAME>`.
- Layout: **header(80) + UTF-8 body + footer(20)**.
- header = type-marker(8) + body-length(LE u32) + date `DF 07 0A 0F` + zeros → 80.
- type markers: TXT `01 00 00 00 00 00 01 00`, DAT `01 00 01 00 00 00 00 00`, GRP `01 00 01 00 00 00 02 00`.
- footer = `HMAC-SHA1(KEY, header+body)`, KEY = `nqmby+e9S?{%U*-V]51n%^xZMk8>b{?x]&?(NmmV[,g85:%6Sqd"'U")/8u77UL2`.
- on-disk name = type-prefix + in-SB name: TXT→`T`, DAT/GRP→`B`. **Programs are TXT files**, so
  program "P" is on-disk `TP` and loads via `LOAD"PRG0:P"`. Result `TXT:O` is on-disk `TO`.

Source for the markers/prefixes/key: nnn1590/lpp-3ds-sbfm (`romfs/index.lua`, the SmileBASIC
File Manager).

## Pitfalls
- `LOAD"TXT:name"` needs an output variable (→ "Illegal function call"); use `LOAD"PRG0:name"` for programs.
- Can't chain `LOAD ... :RUN` on one line (→ "Syntax error"); RUN separately, no `clear_line()` between.
- A file with a bad/absent HMAC footer shows as `?NAME` in FILES and won't load — always use `write_file`.
- Terminal can occlude Azahar; `open -a Azahar` right before capture/click. Don't hammer the RPC.
