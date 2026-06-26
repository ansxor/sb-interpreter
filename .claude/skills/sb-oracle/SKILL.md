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
**Both graphics paths work and are verified:** GRP-page capture (`capture_grp`) and composite
screenshot capture (`capture_composite` / `capture_screen`).

## Quick start

```bash
cd .claude/skills/sb-oracle/tools
python3 run_case.py ready                          # STEP 0: tap SMILE -> arm KEY 1-5 + verify -> READY
python3 run_case.py batch cases.txt out.tsv        # RECOMMENDED: FAST harvest (one mega-program)
python3 run_case.py prog 'FLOOR(-2.1)'             # one value case via the key/program path -> -3
python3 run_case.py prog 'MID$("ABCDE",1,2)' str   # one value case, string result -> BC
python3 run_case.py errcase 'A=SQR(-1)'            # error case -> {errored, errnum, errline}
python3 run_case.py progcase 'DIM A[5]' 'LEN(A)'   # multi-statement case (setup -> result) -> 5
python3 run_case.py grp draw.sb out.png top        # GRAPHICS golden: draw -> SAVE GRPn -> PNG
python3 run_case.py screenshot out.png [top|bottom|both]  # grab the CURRENT rendered screen(s)
python3 run_case.py composite prog.sb3 out.png top # COMPOSITE golden: run prog -> screenshot
python3 run_case.py abort [seconds]            # hold START (M) to abort a wedged program
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
`name|expr|str` (string result, no `STR$` wrap), `name|stmt|err` (error case, below),
`name|setup|result|prog` / `…|progstr` (multi-statement case, below), or bare `expr`.

**Multi-statement cases (`prog`/`progstr`).** A case that needs SETUP before its value —
`DIM A[5]` then `LEN(A)`, `MIN`/`MAX` array form, a multi-line `IF…ENDIF` block, a fractional-
`STEP` FOR pass-count, a seeded `RANDOMIZE`-then-`RND` sequence — can't be a single `STR$(expr)`.
Write it `name|<setup>|<result>|prog` (numeric) or `|progstr` (string). `setup` runs first and
may use `:` (native SB single-line multi-statement) or `\n` (escaped, for real newlines like an
IF block); `result` is the captured expression. These run ALONE (their own LOAD+RUN), not in the
value mega-program — where `DIM`/loop setup would collide across cases. E.g.
`min_arr|DIM T[2]:T[0]=50:T[1]=3|MIN(T)|prog` → `3`;
`if_blk|IF 1 THEN\nA=5\nELSE\nA=9\nENDIF|A|prog` → `5`.

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

### Harvesting limits (queue these, don't force them)
- **Aborting a wedged program (`abort`).** A runaway program (`WHILE 1:WEND`, the i32-wrap
  `FOR I%=2147483640 TO 2147483647:NEXT`, etc.) spins forever and produces no result file
  (`run_program` → `None`). Recover it by holding the 3DS **START** button ~3s: Azahar maps
  START to keyboard **`M`** (`qt-config.ini` `button_start="code:77,engine:keyboard"`,
  Qt::Key_M), and holding it aborts the running program back to DIRECT mode. Use
  `run_case.py abort [seconds]` (→ `sb_window.hold_start`, osascript `key down/up "m"` —
  cliclick `kd:` is modifier-only and can't hold a letter). **Verified live:** `WHILE 1:WEND`
  → `progsrc` returns None → `abort` → `ready` → READY (and `1+1`→`2` after, so SB is intact).
  An abort is NOT an error, so ERRNUM/ERRLINE stay unset — read nothing from them; just re-arm.
  Wrap a maybe-wedging probe in `timeout 60` as a backstop: if the abort somehow doesn't fire
  (focus lost, key unmapped), the shell kills the run and a cold `open -a Azahar` relaunch
  (~12s) always recovers. An endless loop is recoverable, not oracle-fatal.
- **END / STOP — clean-halt is indistinguishable from an error-halt** via the `err` harness:
  both stop the sentinel SAVE, so F2 then reads a stale/irrelevant `ERRNUM`. To probe halt
  behavior, use a **file-based stdout-diff** instead: a program like
  `SAVE"TXT:O","BEFORE":END:SAVE"TXT:O","AFTER"` leaves `O=="BEFORE"` iff `END` halted before the
  second SAVE (the first SAVE's dialog must be confirmed before the halt). `END` (not resumable)
  vs `STOP` (CONT-resumable) then needs a CONT tap after the halt to see if the second SAVE runs.
  Not yet automated — track in beads (`bd create`).
- **XON / XOFF and other hardware-feature commands** can pop a feature-confirmation dialog that
  may hang the harness (and need an emulated peripheral). Don't drive these live; spec them from
  docs + disassembly and track in beads.
- **Cross-slot behavior (COMMON / USE, EXEC into another slot)** needs a multi-program-slot
  harness; the single-`P` flow can't express it. Track in beads.

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
- **Start draw programs with `ACLS` (or `GCLS` for the target page).** GRP pages are 512×512
  buffers that persist across runs — without a clear, a prior program's pixels remain on the
  page and the golden captures the union. `ACLS` clears all GRP pages + sprites/BG/console; a
  bare `GCLS` clears just one page. The harness's F5/CLS clears console *text* only.
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
  use the composite screenshot path below.
- Goldens go in `harness/corpus/golden/gfx/*.png` (see `harness/corpus/README.md`).

## Composite capture (O-T6 screenshot path) — `capture_composite` / `run_case.py composite`
For sprite/BG/backdrop/console side-effects that don't write a GRP page — the *composited*
display (all layers) — capture via an Azahar screenshot:
- **Start graphical test programs with `ACLS`.** A screenshot captures *every* layer, so
  residual state from earlier runs — console text, sprites, BG, prior GRP draws — poisons the
  golden. `ACLS` resets all graphics screens to their initial state; the program then redraws
  exactly what the golden should show. (The harness's F5/CLS between runs clears console *text*
  only, not sprites/BG/GRP, so without `ACLS` a prior program's draws bleed into the capture.)
  E.g. write `ACLS:BACKCOLOR RGB(255,0,0)` not bare `BACKCOLOR RGB(255,0,0)`.
- **End graphical test programs with an infinite loop** (`WHILE 1:WEND`) so `capture_composite`
  screenshots a *clean running frame*. A program that finishes returns to DIRECT mode and SB
  renders the `Ok` prompt (top-left) before the grab — that console text bleeds into the golden
  (verified: bare `BACKCOLOR` → 46 stray white px at (0–13,0–15) = the `Ok` glyph). The loop
  holds the program at its rendered state; `capture_composite` grabs it; then `run_case.py abort`
  recovers (hold START/M). Pattern: `ACLS:<draw>:WHILE 1:WEND`. Verified: the looped form is
  100% uniform red, byte-stable across re-grabs (0/96000 px diff).
- **The Ctrl+P chord is DEAD.** It's registered in Azahar's config but the keyboard chord never
  fires the action (the render widget doesn't take chords even when the window is frontmost —
  verified: the screenshots dir stayed empty). So `key_combo("ctrl","p")` is not used.
- **Drive the menu item instead:** `sb_window.capture_screenshot_menu()` clicks
  **Tools → "Capture Screenshot"** via System Events. This reliably lands a PNG.
- **`capture_screen(out_png, screen=)`** grabs the *current* screen (no program run): the landed
  PNG is `400×480` RGB (both screens stacked: top rows 0–239, bottom 240–479); we split the
  requested `screen` (`top`/`bottom`/`both`) and re-encode as `400×240` RGBA (color type 6, same
  shape as `golden/gfx/*.png`) so the CI diff path (`harness/diff/png_util.decode_rgba`) accepts
  it. Alpha is padded to `0xFF` (screenshots are fully opaque).
- **`capture_composite(program, out_png, screen=)`** is the run-then-screenshot flow (mirrors
  `capture_grp`'s F1/F4 load+run but **without SAVE** — the screenshot captures the rendered
  layout directly, so no extdata result file and no SAVE-dialog handling). A `RUN_SETTLE` beat
  lets sprites/BG/anims render before the grab.
- **Verified on real SB 3.6.0:** `BACKCOLOR RGB(255,0,0)` → a uniformly `(255,0,0)` opaque top
  screen (the backdrop composite); `SPSET 0,0:SPOFS 0,50,50:SPSHOW 0` → the sprite's dark-red
  template pixel over the black backdrop at (50,50). Round-trips through `decode_rgba` cleanly.
- **XSCREEN / both screens:** screenshot captures both screens stacked; pass `screen="bottom"`
  for the touch screen, `"both"` for the full 400×480. Don't use XSCREEN 2/3/4 for capture —
  2/3 swap the touch screen to a keyboard and 4 forbids DIRECT mode, stranding the oracle's taps.
- Composite goldens go in `harness/corpus/golden/composite/*.png` (oracle-truth storage; no
  hermetic CI pixel-diff gate yet — `sb-run` can't render the full composited framebuffer, which
  is a follow-up bead).

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
