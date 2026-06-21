# PRD — Task Breakdown

Canonical task list for the SmileBASIC 3.6.0 interpreter. **This file is only tasks.**
Design context, references, and acceptance criteria live in per-milestone documents
under `prd/` (start at `prd/README.md`). Task IDs here match those docs.

**Legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `→` depends on.

| Milestone | Goal | Doc | Status |
|---|---|---|---|
| M0 | Scaffolding & spec pipeline | `prd/M0.md` | ✅ done |
| M1 | Core VM + a real window | `prd/M1.md` | ⬜ |
| M2 | Graphics (GRP + compositor) | `prd/M2.md` | ⬜ |
| M3 | Sprites & BG | `prd/M3.md` | ⬜ |
| M4 | Input & timing | `prd/M4.md` | ⬜ |
| M5 | Audio (MML) | `prd/M5.md` | ⬜ |
| M6 | Files, projects, system, stubs | `prd/M6.md` | ⬜ |
| M7 | Hardening | `prd/M7.md` | ⬜ |
| O | Emulator-oracle bring-up | `prd/oracle.md` | ⬜ |

---

## M0 — Scaffolding & spec pipeline ✅
- [x] M0-T1 Scaffold Rust workspace + 6 crates (build native + wasm32)
- [x] M0-T2 Move harness tools into `tools/`
- [x] M0-T3 Spec ingestion `tools/gen_specs.py` → 248 instruction specs + reference tables
- [x] M0-T4 `sb-spec` loader + coverage bin + test-overlay merge
- [x] M0-T5 Harness skeleton (oracle/diff/fuzz/harvest/corpus) + ported goldens
- [x] M0-T6 CI (deterministic replay only) + `.gitignore` + git init

## M1 — Core VM + a real window
- [x] M1-T1 Lexer — tokenize SB source (TokenType, Token, SourceLocation; `&H`/`&B`, `$`/`%`/`#` suffixes, comments, `TRUE`/`FALSE`, 2-char ops)
- [x] M1-T2 AST — expression/statement node types → M1-T1
- [ ] M1-T3 Parser — recursive descent + precedence-climbing + constant folding → M1-T2
- [ ] M1-T4 Value/Array completion — 1–4D arrays, references, int↔double coercion rules → (M0 value.rs)
- [ ] M1-T5 Bytecode + Compiler — Code opcodes, AST→bytecode, scopes, labels, DATA table → M1-T3, M1-T4
- [ ] M1-T6 VM — stack machine, frames, 4 slots + COMMON DEF, dispatch loop → M1-T5
- [ ] M1-T7 Builtin registration macro + math/string builtins → M1-T6
- [ ] M1-T8 Control-flow + console builtins (PRINT/`?`, LOCATE, COLOR, CLS, INPUT, LINPUT) → M1-T7, M1-T10
- [ ] M1-T9 TinyMT RNG (port `tinymt32.d` + 8-engine wrapper; RND/RNDF/RANDOMIZE; RNDF double-draw) → M1-T7
- [ ] M1-T10 Console model — 50×30 grid, attributes, font, TABSTEP; render to framebuffer → (M0 sb-render)
- [ ] M1-T11 Headless runner `sb-run` — run a `.sb3`, emit console text to stdout (for replay) → M1-T8
- [ ] M1-T12 Window — native (winit + pixels) + wasm (canvas/WebGL) blit of framebuffer, 60 fps → M1-T10
- [ ] M1-T13 Error model — SbError propagation, ERRNUM/ERRLINE sysvars, messages per spec → M1-T6
- [ ] M1-T14 Conformance wiring — execute `spec/tests/` + `corpus/cases` via sb-core in `cargo test`; `ASSERT__` test builtin; run `otya_test.sb3` → M1-T11

## M2 — Graphics (GRP + compositor)
- [ ] M2-T1 GRP page model — GPAGE, GCLS, GCOLOR, GPRIO, GCLIP, RGB/RGBREAD/GSPOIT
- [ ] M2-T2 Drawing primitives — GPSET, GLINE, GBOX, GFILL, GCIRCLE, GTRI, GPAINT → M2-T1
- [ ] M2-T3 Bitmap ops — GCOPY, GLOAD, GSAVE → M2-T1
- [ ] M2-T4 5-layer compositor — backdrop→GRP→BG→sprite→console, Z order, RGBA5551 math → M2-T2
- [ ] M2-T5 RE pixel/color math from disassembly; harvest golden PNGs; pixel-diff replay → M2-T4, O-T6

## M3 — Sprites & BG
- [ ] M3-T1 Sprite core — SPSET/SPCLR/SPSHOW/SPHIDE/SPOFS/SPCHR/SPSCALE/SPROT/SPCOLOR/SPHOME/SPPAGE/SPUSED → M2-T4
- [ ] M3-T2 Sprite animation/link/vars — SPANIM/SPSTART/SPSTOP/SPFUNC(+CALLIDX)/SPVAR/SPLINK/SPUNLINK → M3-T1
- [ ] M3-T3 Sprite collision — SPCOL/SPCHK/SPHITSP/SPHITRC/SPHITINFO/SPCOLVEC; SPDEF (+`spdef.csv`) → M3-T1
- [ ] M3-T4 BG core — BGPUT/BGGET/BGFILL/BGOFS/BGROT/BGSCALE/BGCOLOR/BGSCREEN/BGCLIP/BGSHOW/BGHIDE/BGHOME/BGPAGE/BGCLR → M2-T4
- [ ] M3-T5 BG animation/extras — BGANIM/BGFUNC/BGSTART/BGSTOP/BGVAR/BGCHK/BGCOORD/BGCOPY/BGLOAD/BGSAVE → M3-T4
- [ ] M3-T6 Composite sprites + BG into framebuffer; golden PNG diffs → M3-T1, M3-T4

## M4 — Input & timing
- [ ] M4-T1 Buttons/sticks — BUTTON, STICK, STICKEX, BREPEAT
- [ ] M4-T2 Touch + keyboard — TOUCH, KEY, INKEY$
- [ ] M4-T3 Frame timing — VSYNC, WAIT, MAINCNT; 60 fps main loop → M1-T12
- [ ] M4-T4 Display config — XSCREEN, DISPLAY, VISIBLE, WIDTH; HARDWARE sysvar
- [ ] M4-T5 Host input mapping — native (keyboard/gamepad) + wasm (keyboard/gamepad) → SB input state → M4-T1, M4-T2

## M5 — Audio (MML)
- [ ] M5-T1 MML parser — per `spec/reference/mml.yaml` (channels, lengths, octaves, envelopes, repeats, macros)
- [ ] M5-T2 Synth engine — instruments @0–@127, drums @128/@129, PSG @144–@151, user waveforms @224–@255, envelopes → M5-T1
- [ ] M5-T3 BGM commands — BGMPLAY/BGMSET/BGMSETD/BGMSTOP/BGMCHK/BGMVAR/BGMVOL/BGMCLEAR → M5-T2
- [ ] M5-T4 SFX/voice — BEEP, TALK/TALKCHK/TALKSTOP, EFCSET/EFCON/EFCOFF/EFCWET, WAVSET/WAVSETA → M5-T2
- [ ] M5-T5 Audio backend — cpal (native) / WebAudio (wasm) → M5-T2
- [ ] M5-T6 RE exact sample rate/timing; harvest golden WAVs; sample/spectral-diff replay → M5-T5, O-T7

## M6 — Files, projects, system, faithful stubs
- [ ] M6-T1 Storage abstraction — native FS / wasm IndexedDB; extdata-compatible layout
- [ ] M6-T2 File commands — SAVE/LOAD/FILES/DELETE/RENAME/COPY/CHKFILE/PROJECT → M6-T1
- [ ] M6-T3 System variables — DATE$/TIME$/MAINCNT/VERSION/FREEMEM model/RESULT/etc. per spec
- [ ] M6-T4 Source-edit — PRGEDIT/PRGGET$/PRGSET/PRGINS/PRGDEL/PRGNAME$/PRGSIZE; error 38
- [ ] M6-T5 Misc + faithful limitation stubs — DIALOG/FONTDEF/CLIPBOARD; MIC/MOTION/MP behavior + errors 36/37/43/44/45
- [ ] M6-T6 Multi-slot semantics — EXEC/USE/CALL/COMMON DEF across slots → M1-T6

## M7 — Hardening
- [ ] M7-T1 Fuzzing campaign — seeded grammar-aware generator, differential vs oracle, minimize + promote findings → O-T8
- [ ] M7-T2 hw_verified push — oracle-harvest expects across the spec set; raise confidence → O-T8
- [ ] M7-T3 Exact error strings — reconcile messages/numbers vs disassembly (`0x1E965C`+)
- [ ] M7-T4 Float formatting — exact STR$/PRINT double→string algorithm (RE from disassembly)
- [ ] M7-T5 Overflow/precision corners + performance pass

## O — Emulator-oracle bring-up (parallel, independent of interpreter)
- [ ] O-T1 Confirm Citra/Azahar RPC connects to running SB3 (`process_list`, `read_memory(0x100000,4)`)
- [ ] O-T2 Autorun — auto-start a program in SB3 under emulation
- [ ] O-T3 extdata container format — inject programs/files into SB extdata
- [ ] O-T4 stdout capture — CHKCHR grid scrape vs console-memory read
- [ ] O-T5 ERRNUM/ERRLINE address RE + error capture → O-T1
- [ ] O-T6 Framebuffer address + pixel format RE (top/bottom) → O-T1
- [ ] O-T7 Audio capture from emulator → O-T1
- [ ] O-T8 `harvest.py` end-to-end — capture → write `spec/tests/` overlays + golden media → O-T2, O-T4, O-T5
