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
python3 run_case.py ready                          # STEP 0: tap SMILE -> arm KEY 1-5 + verify -> READY
python3 run_case.py batch cases.txt out.tsv        # RECOMMENDED: FAST harvest (one mega-program)
python3 run_case.py prog 'FLOOR(-2.1)'             # one value case via the key/program path -> -3
python3 run_case.py prog 'MID$("ABCDE",1,2)' str   # one value case, string result -> BC
python3 run_case.py errcase 'A=SQR(-1)'            # error case -> {errored, errnum, errline}
python3 run_case.py grp draw.sb out.png top        # GRAPHICS golden: draw -> SAVE GRPn -> PNG
python3 run_case.py screenshot out.png             # COMPOSITE golden (sprites/BG): Ctrl+P
```
Run `ready` FIRST — it taps the SMILE button (which runs OBOOT to arm the five function keys)
and proves the SAVE->dialog->disk path by writing `__OK__` to result file `O`. A `sb_window.py
bounds` that returns coords is NOT proof of readiness; a green `ready` (O=="__OK__") is.

**Input is via function keys, never typed.** The whole harvest drives SB through five armed
KEY slots — `F1`=LOAD, `F4`=RUN, `F2`=save-error, `F3`=save-`__OK__`, `F5`=CLS — each a single
calibrated tap at a fixed screen coordinate. Char-by-char typing (the old flakiness source) is
gone. The keys are armed once by the **SMILE button** (see Setup); `run_case.py` only taps.

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
0. **Launch + arm:** open Azahar and get SB onto the **DIRECT-mode screen** (keyboard visible).
   Then `run_case.py ready` taps SMILE to arm the keys and verifies via `O=="__OK__"`.
1. **Assign OBOOT to the SMILE button — one-time, manual, in SB's settings.** `smile_boot.sb`
   (program **`OBOOT`**, already injected to extdata by `sb_extdata.write_file`) is the bootstrap:
   tapping SMILE loads+runs it, which sets `KEY 1`-`KEY 5` and SAVEs `__OK__` to `O`. Assign
   program `OBOOT` to SMILE once; thereafter `ready` re-arms in one tap. (Re-inject OBOOT after
   editing `smile_boot.sb`: `python3 -c 'import sb_extdata as X; X.write_file("OBOOT", open("smile_boot.sb").read(), "TXT")'`.)
2. Screen Recording + Accessibility granted to the terminal app; `cliclick` installed.

## Mechanism
- **Raise window:** `open -a Azahar` (osascript `activate` is unreliable). Window pinned to
  pos(60,80) size(400,539) for stable coordinates.
- **Touch:** `cliclick` at screen `(60+wx, 80+wy)`. Keymap (`keymap.json`) holds every key plus
  the named taps **`F1`-`F5`** (function-key row, y≈278), **`SMILE`** (223,495), **`YES`** (318,488).
- **The five armed keys (set by OBOOT via the SMILE button):**
  `F1`=`LOAD"PRG0:P",0` · `F4`=`RUN` · `F2`=`SAVE"TXT:O",STR$(ERRNUM)+CHR$(9)+STR$(ERRLINE)` ·
  `F3`=`SAVE"TXT:O","__OK__"` · `F5`=`CLS`. Each KEY macro is ONE line ending in `CHR$(13)`. LOAD
  and RUN are **separate keys** — a KEY macro can't hold a multi-line LOAD+RUN (an embedded
  `CHR$(10)`/`CHR$(13)` does not reliably run both lines on hardware).
- **Run a program:** `sb_extdata.write_file()` writes the program to extdata as slot `P`, then
  **delete result file `O`** (so "O exists" = fresh), tap **F5** (CLS), **F1** (load), **F4**
  (run). The program SAVEs its result; read `O` from disk. No long typing — only the F-keys.
- **SAVE-dialog handling (`W.confirm_dialogs()`):** a SmileBASIC SAVE is a **two-dialog**
  sequence — `Confirm · Write file` (tap **YES**, file is written) then `Information · Write file`
  (tap **OK**, same screen position). LOAD from a key slot adds a one-tap load dialog (the `,0`
  auto-dismisses only when *typed*, not from a key). `confirm_dialogs()` taps the YES/OK button
  until **no dialog is on screen** (it samples bottom-screen brightness: dialog body ≥158, keyboard
  ≤75) — so every save self-closes. NEVER tap YES speculatively to "clear a stale dialog": when
  none is open that tap lands on a key and injects junk. Each op closes its own dialogs instead.
- **Mega-program (the fast harvest):** `batch` writes ONE program — `R$=""` then a
  `R$=R$+"<name>"+CHR$(9)+STR$(<expr>)+CHR$(10)` line per case, then `SAVE"TXT:O",R$` — so all
  results come back in a single file (`name<TAB>value`, LF-separated). One F1+F4 for the whole
  slice. No in-program error handling exists in SB, so a halting case yields no file → `harvest`
  bisects to find it.
- **Error capture (`run_error_case`) — reliable now:** program is `<stmt>` + a
  `SAVE"TXT:O","__OK__"` sentinel. Delete `O`, F1+F4. If `O=="__OK__"` it didn't raise (NOERR);
  if `O` is **absent** the stmt halted before the sentinel → tap **F2** to SAVE
  `ERRNUM`/`ERRLINE` (set by this run's halt, read in DIRECT mode). The delete-first kills the
  old `errline=0` stale-read ghost. Freeze `errnum` into the spec test's `expect.error`.

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

## Graphics capture (O-T6) — `capture_grp` / `run_case.py grp`
Deterministic pixel goldens **without screenshots**: a program draws to a GRP page, `SAVE"GRPn:NAME"`
writes it to disk, and `sb_grp.decode_grp` decodes it to RGBA → PNG.
- **GRP file body** = 28-byte internal header + raw pixels. Header: `"PCBN"`+`"0001"`, then
  `width`(u32 LE @12)·`height`(u32 LE @16) = **512×512** (the full page; off-screen region included).
- **Pixels:** 16-bit **RGBA5551** LE, row-major, top-left origin. Bits MSB→LSB `R:5 G:5 B:5 A:1`
  (A = bit 0, 1=opaque). 5→8-bit expand = `v<<3` (matches sb-render `expand5` / `#WHITE=&HFFF8F8F8`).
- **Verified on real SB 3.6.0:** red=`F801`, green=`07C1`, blue=`003F`, black=`0001`; a full
  draw (square+circle+line) round-trips to the exact PNG.
- **XSCREEN / both screens:** GRP pages (GRP0–5) are 512×512 buffers **independent of XSCREEN /
  display mode** — this reads the page *buffer* off disk, not a screen, so the mode can't corrupt
  it. For content on two screens, capture **each page** (`capture_grp(page=N)`, verified: GRP0 + GRP1
  captured independently). **Don't change XSCREEN just to capture** — page content is mode-independent,
  and XSCREEN 2/3 swap the touch screen to a keyboard / XSCREEN 4 forbids DIRECT mode, which would
  strand the oracle's taps. Draw in the default mode, capture per page.
- **One SAVE per run:** two `SAVE`s in one program fight over the confirm dialog — call `capture_grp`
  once per page instead.
- **Composite / actual display** (sprites + BG + XSCREEN 4 combined + 3D) isn't in a GRP page →
  use `capture_screen` (Azahar **Ctrl+P** screenshot of the rendered layout, both screens).
- Goldens go in `harness/corpus/golden/gfx/*.png` (see `harness/corpus/README.md`).

## Audio (O-T7) — reference only, NOT a deterministic golden
SB has no render-audio-to-file, and emulator audio is real-time (mixing/timing/sample-rate
dependent), so there is **no sample-exact, replayable audio golden** like the gfx PNGs. The
deterministic contract for M5 is **MML → note-events + synth params** (docs + disassembly, no
emulator). `sb_audio.py` is a best-effort *reference* only:
- `sb_audio.py extract <video> [out.wav]` — pull the audio track from a dump via ffmpeg (verified).
- `capture_audio(program, out_wav, seconds, run_trigger)` — drives Azahar `Tools > Dump Video`
  (osascript) around a program run, then extracts WAV. **Live-UNTESTED** (kept off the running
  oracle to avoid wedging it on the save dialog); for manual ear-checks / loose spectral compare,
  never a committed CI fixture.

## Pitfalls
- **A SAVE is two dialogs:** `Confirm · Write file` (→ YES, file written) then `Information ·
  Write file` (→ OK, same spot). Closing only the first orphans the second; the next op then
  breaks. Use `W.confirm_dialogs()`, which taps until the screen is dialog-free.
- **Never speculatively tap YES** to clear a maybe-stale dialog: with none open the tap hits a
  key and injects junk (e.g. a stray `KEY 1,"E"…`). Make each op close its own dialogs.
- **`,0` auto-dismisses the load dialog only when TYPED, not from a KEY slot.** So F1 (`LOAD…,0`)
  still raises a one-tap load dialog — `_load_prog()` clears it with `confirm_dialogs()`.
- **A KEY macro is one line.** An embedded `CHR$(10)` or `CHR$(13)` does NOT reliably run a
  two-line `LOAD`+`RUN` from one key (the CR truncates the macro; the LF buffers oddly). Keep
  LOAD (F1) and RUN (F4) as separate keys.
- **Delete the result file `O` before each run** so "O exists" means a fresh result — this is
  what kills the old `errnum=0`/`errline=0` stale-read artifact.
- `LOAD"TXT:name"` needs an output variable (→ "Illegal function call"); use `LOAD"PRG0:name"` for programs.
- Can't chain `LOAD ... :RUN` on one line (→ "Syntax error"); LOAD (F1) and RUN (F4) are separate taps.
- A file with a bad/absent HMAC footer shows as `?NAME` in FILES and won't load — always use `write_file`.
- Terminal can occlude Azahar; `open -a Azahar` right before capture/click. Don't hammer the RPC.
