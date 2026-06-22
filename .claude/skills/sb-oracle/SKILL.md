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
python3 run_case.py batch cases.txt out.tsv        # RECOMMENDED: many `name|expr`, incremental + resumable
python3 run_case.py prog 'FLOOR(-2.1)'             # one case via the program-file path -> -3
python3 run_case.py expr 'MID$("ABCDE",2,3)' str   # one typed case, string -> BCD
```
Run `ready` FIRST — it cold-starts Azahar and confirms SB is usable, so cases don't each eat a
timeout (a `sb_window.py bounds` that returns coords is NOT proof of readiness). Prefer `batch`
(one process, no backgrounding/sleep) over many single calls — the harness blocks `sleep N; cmd`.
**Always pass an OUTFILE** to `batch` (each `name<TAB>result` is appended + flushed as it lands):
the oracle is slow (~tens of seconds/case), so a run that's killed mid-harvest (timeout, out of
credits) keeps everything so far — re-running `batch` with the same OUTFILE skips OK rows and
retries only `ERROR` ones. Harvest a small slice, fold it into the spec, then harvest the next.
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
