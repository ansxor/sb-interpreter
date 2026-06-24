# PRD — Task Breakdown

Canonical task list for the SmileBASIC 3.6.0 interpreter. **This file is only tasks.**
Design context, references, and acceptance criteria live in per-milestone documents
under `prd/` (start at `prd/README.md`). Task IDs here match those docs.

**Legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `→` depends on.

> **Active priority: S (spec build-out) + O (oracle).** The doc-only specs were deleted —
> they were built from `sb-docs` alone, not from all sources. The real contract is built
> from **docs + disassembly + osb cross-check + oracle (hw_verified)**. Interpreter
> implementation (M1–M7) is **gated on the spec suite existing** for the relevant category.

| Milestone | Goal | Doc | Status |
|---|---|---|---|
| M0 | Scaffolding & spec pipeline | `prd/M0.md` | ✅ done |
| **S** | **Spec build-out (all sources)** | `prd/specs.md` | 🔥 active |
| **O** | **Oracle engine — `sb-oracle` skill** | `prd/oracle.md` | 🔥 value/errnum/graphics harvest work; audio = MML-event specs (no emulator golden) + ref capture |
| M1 | Core VM + a real window | `prd/M1.md` | ✅ done (T1–T14; conformance gate green, native + wasm) |
| M2 | Graphics (GRP + compositor) | `prd/M2.md` | ✅ done (T1–T5; GRP model, primitives, bitmap ops, compositor, hw_verified golden pixel-diff) |
| M3 | Sprites & BG | `prd/M3.md` | ✅ done (T1–T6; sprite/BG state + collision/anim/transforms, full compositor stack with Z-interleaving; composite pixel-exactness queued O-T6) |
| M4 | Input & timing | `prd/M4.md` | ✅ done (T1–T5; input state + 60fps clock + host keymap wired native + wasm; live-program input awaits the frame-yielding model) |
| M5 | Audio (MML) | `prd/M5.md` | 🔧 in progress (T1 MML parser, T2 synth, T3 BGM, T4 SFX/voice, T5 audio backend done; T6 golden-WAV harvest todo — deferred, no deterministic audio golden per O-T7) |
| M6 | Files, projects, system, stubs | `prd/M6.md` | ✅ done (T1 storage, T2 file commands, T3 system variables, T4 PRG* source-edit, T5 misc/limitation stubs, T6 multi-slot — 4 slots + COMMON DEF, EXEC/USE/CALL, cross-slot scoping, CALLIDX callbacks; refinements oracle-pending) |
| M7 | Hardening | `prd/M7.md` | ⬜ |

---

## S — Spec build-out (the contract; from docs + disassembly + osb + oracle)
Each instruction spec gets: typed signature (arg types/ranges/defaults), precise semantics,
error conditions (errnum), and test cases (code → expect) with honest per-source confidence.
A category is done when every instruction in it is specced with cases, and oracle-verifiable
cases are harvested (`hw_verified`) or queued in `HARVEST_QUEUE.md`.

Tasks are **sliced to ≤6 instructions** so one Ralph run finishes a slice end-to-end (spec
from docs+disasm+osb, then incremental oracle harvest) inside one context/credit window. A
`S-Tn` group is done when all its slices are `[x]`. **All S-T* slices depend on S-T0** (done)
and name the instructions they cover inline.

- [x] S-T0 Spec schema v2 + authoring guide — v2 contract (typed sigs/ranges/errors/cases) + 4-source process in `prd/specs.md`; `sb-spec` structs updated; **concept-spec** kind (Markdown) added; FLOOR exemplar + screen-and-color-model exemplar written

#### S-T1 Mathematics (27) → S-T0
- [x] S-T1a Rounding — FLOOR · ROUND · CEIL
- [x] S-T1b Sign & classify — ABS · SGN · CLASSIFY
- [x] S-T1c Powers/roots/log — SQR · POW · EXP · LOG
- [x] S-T1d Trigonometry — SIN · COS · TAN · ASIN · ACOS · ATAN
- [x] S-T1e Hyperbolic & angle — SINH · COSH · TANH · DEG · RAD · PI
- [x] S-T1f Min/max & RNG — MIN · MAX · RND · RNDF · RANDOMIZE

#### S-T2 Strings (12) → S-T0
- [x] S-T2a Extract — LEFT$ · RIGHT$ · MID$ · SUBST$
- [x] S-T2b Convert — STR$ · VAL · HEX$ · FORMAT$
- [x] S-T2c Char/search/len — ASC · CHR$ · INSTR · LEN

#### S-T3 Control + Advanced control (27) → S-T0
- [x] S-T3a Conditionals — IF · THEN · ELSE · ELSEIF · ENDIF
- [x] S-T3b Counted loops — FOR · NEXT · TO · STEP
- [x] S-T3c While/repeat & flow — WHILE · WEND · REPEAT · UNTIL · BREAK · CONTINUE
- [x] S-T3d Branch & halt — GOTO · GOSUB · RETURN · ON · OUT · END · STOP
- [x] S-T3e Advanced control — CALL · COMMON · DEF · XON · XOFF

#### S-T4 Variables/arrays + Data-ops (27) → S-T0
- [x] S-T4a Declaration & inc — VAR · DIM · DEC · INC · SWAP
- [x] S-T4b Array stack/queue — PUSH · POP · SHIFT · UNSHIFT
- [x] S-T4c Array ops — COPY · FILL · SORT · RSORT
- [x] S-T4d DATA/READ — DATA · READ · RESTORE · REM
- [x] S-T4e Read helpers & checks — DTREAD · TMREAD · CHKCALL · CHKLABEL · CHKVAR
- [x] S-T4f Misc data-ops — DIALOG · KEY · OPTION · VSYNC · WAIT

#### S-T5 Console I/O (12) → S-T0
- [x] S-T5a Output — PRINT · LOCATE · COLOR · CLS
- [x] S-T5b Input — INPUT · LINPUT · INKEY$
- [x] S-T5c Attributes & font — ATTR · CHKCHR · FONTDEF · SCROLL · WIDTH

#### S-T6 Bit-ops + operators (5) → S-T0
- [x] S-T6a Bit/logic operators — AND · OR · XOR · DIV · MOD

#### S-T7 Graphics (19) → S-T0  (no framebuffer harvest yet → O-T6; spec from docs+disasm)
- [x] S-T7a Page/clip/color — GPAGE · GCLS · GCLIP · GPRIO · GCOLOR
- [x] S-T7b Primitives — GPSET · GLINE · GBOX · GTRI · GCIRCLE
- [x] S-T7c Fill & char — GFILL · GPAINT · GPUTCHR
- [x] S-T7d Buffer copy/load/save — GCOPY · GLOAD · GSAVE
- [x] S-T7e Color read — GSPOIT · RGB · RGBREAD

#### S-T8 Sprites (27) → S-T0
- [x] S-T8a Lifecycle — SPSET · SPCLR · SPSHOW · SPHIDE · SPPAGE
- [x] S-T8b Transform — SPOFS · SPROT · SPSCALE · SPHOME · SPCHR
- [x] S-T8c Animation & link — SPANIM · SPSTART · SPSTOP · SPLINK · SPUNLINK
- [x] S-T8d Collision — SPCOL · SPCOLVEC · SPHITSP · SPHITRC · SPHITINFO
- [x] S-T8e Vars/funcs/state — SPVAR · SPFUNC · SPDEF · SPCHK · SPUSED · SPCLIP · SPCOLOR

#### S-T9 BG (24) → S-T0
- [x] S-T9a Setup — BGSCREEN · BGPAGE · BGCLR · BGSHOW · BGHIDE
- [x] S-T9b Tiles — BGPUT · BGFILL · BGGET · BGCOPY · BGCLIP
- [x] S-T9c Transform — BGOFS · BGROT · BGSCALE · BGHOME · BGCOORD
- [x] S-T9d Animation & state — BGANIM · BGSTART · BGSTOP · BGVAR · BGFUNC · BGCHK
- [x] S-T9e Load/save/color — BGLOAD · BGSAVE · BGCOLOR

#### S-T10 Sound (18) → S-T0  (MML grammar = S-C5; no audio harvest yet → O-T7)
- [x] S-T10a BGM playback — BGMPLAY · BGMSTOP · BGMCHK · BGMVOL · BGMVAR
- [x] S-T10b BGM setup — BGMSET · BGMSETD · BGMCLEAR · BEEP
- [x] S-T10c Effects — EFCON · EFCOFF · EFCSET · EFCWET
- [x] S-T10d Voice & wave — TALK · TALKCHK · TALKSTOP · WAVSET · WAVSETA

#### S-T11 Various input + Screen control (20) → S-T0
- [x] S-T11a Buttons & sticks — BUTTON · BREPEAT · STICK · STICKEX
- [x] S-T11b Touch & motion — TOUCH · ACCEL · GYROA · GYROV · GYROSYNC
- [x] S-T11c Microphone — MICSTART · MICSTOP · MICDATA · MICSAVE
- [x] S-T11d Screen control — ACLS · BACKCOLOR · DISPLAY · VISIBLE · XSCREEN
- [x] S-T11e Fade — FADE · FADECHK

#### S-T12 Files + Source-manip + DIRECT-mode (22) → S-T0
- [x] S-T12a File I/O — LOAD · SAVE · FILES · DELETE
- [x] S-T12b File management — CHKFILE · RENAME · USE · EXEC
- [x] S-T12c Source read — PRGGET$ · PRGNAME$ · PRGSIZE
- [x] S-T12d Source edit — PRGSET · PRGINS · PRGDEL · PRGEDIT
- [x] S-T12e DIRECT-mode — RUN · CONT · NEW · CLEAR · LIST · BACKTRACE · PROJECT

#### S-T13 Wireless (8) → S-T0
- [x] S-T13a Session — MPSTART · MPEND · MPSET · MPSTAT
- [x] S-T13b Messaging — MPSEND · MPRECV · MPGET · MPNAME$

#### S-T14 Verify reference tables (vs disassembly + oracle) → O-T4
- [x] S-T14a Error table — `spec/reference/errors.yaml` vs disasm error strings (@≈0x1E965C) + oracle → O-T5
- [x] S-T14b System variables — `spec/reference/sysvars.yaml` vs disasm sysvar addrs + oracle
- [x] S-T14c Built-in constants — `spec/reference/constants.yaml` vs disasm constant names + oracle (all 79 hw_verified; corrected 7 doc errors: #BLUE/#CYAN, #ZL/#ZR swap, #BGROT90/180/270)

### S-C — Concept specs (architecture/models; Markdown in `spec/concepts/`, see prd/specs.md)
- [x] S-C1 execution-model — lexer/parser/compiler/VM, 4 slots + COMMON, frame layout · governs M1 (`spec/concepts/execution-model.md`; docs + osb structural; frame layout/identifier-class/`^`-rank queued for disasm+oracle)
- [x] S-C2 screen-and-color-model — layers/Z/RGBA5551 · governs M2, O-T6 (`spec/concepts/screen-and-color-model.md`; disassembled RGBA5551 device-pixel bit layout R[15:11]G[10:6]B[5:1]A[0] from pixel-read helper FUN_00191dfc @0x191e40 — masks 0xf8/0xf800/0xf80000 + shifts lsl#2/#5/#8 + tst#1 alpha prove 5→8 expansion is LEFT-SHIFT-ONLY; hw_verified via constants #WHITE=&HFFF8F8F8 (S-T14c) + GSPOIT post-draw round-trip RGB(255,0,0)→-524288 / RGB(0,100,0)→&HFF006000 / off-page→0 (s_c2); GRP page = 512×512 RGBA5551 LE (O-T6). Composite per-layer Z defaults/blending queued → O-T6 composite)
- [x] S-C3 sprite-bg-model — attributes/animation/collision/tilemaps · governs M3 (`spec/concepts/sprite-bg-model.md`; docs + disassembled instruction specs + hw_verified constant bits; mid-anim bits/SPVAR OOR/Z-tiebreak queued)
- [x] S-C4 frame-and-timing-model — VSYNC/WAIT/MAINCNT, 60 fps · governs M4 (`spec/concepts/frame-and-timing-model.md`; disassembled: one global frame counter `[0x315ec0]` read by MAINCNT getter + WAIT, per-program lastVsync `[0x315ee8]` for VSYNC, `swi 0xa` frame yield; MAINCNT reset/VSYNC-catchup queued)
- [x] S-C5 mml-grammar — the full MML language · governs M5 (`spec/concepts/mml-grammar.md`; docs SB3 ref+manual, SB4 cross-check; disassembled BGMPLAY handler @0x1a2d54: argcount 1-3 else errnum 4 · MML validate bl 0x1d44d8→0x1d475c fail→errnum 47 · preset BGM 0-42, user 128-255; corpus-surfaced @V velocity + @256+ SFX bank; tick base/T→frames + @V scaling queued)
- [x] S-C6 file-and-extdata-format — projects/resources/extdata layout · governs M6, O-T3 (`spec/concepts/file-and-extdata-format.md`; hw_verified extdata container header/body/HMAC footer + PCBN GRP layout via sb-oracle round-trip O-T3/T4/T6; disassembled SAVE handler @0x18e7d4 resource-name parse + errnum 3/4/10 sites; documented project/active-project model; PETC corpus container; DAT-array tagging/GRPF/header-date queued)
- [x] S-C7 error-model — errnum/ERRLINE, halt/CONT semantics · governs M1, O-T5 (`spec/concepts/error-model.md`; disassembled errnum→string formatter FUN_001e94a8 @0x1e94a8 — range-guard (errnum-1)≤55, table @0x3054f8→pool @0x2e965c, "Internal Error" fallback, "(detail)" append, store errnum→*[0x315d6c]; errors.yaml 0..55 + sysvars ERRNUM/ERRLINE/ERRPRG read-only; hw_verified persistence into DIRECT post-halt O-T5/S-T14a; NO error trapping; STOP/END/BREAK/error distinguished; CONT/RUN DIRECT-keyword resume index-dispatched = hypothesis; resumable-error set/ERRPRG cross-slot/clear-points queued)

## O — Oracle engine — implemented as the `.claude/skills/sb-oracle/` skill (Azahar + cliclick + extdata)
- [x] O-T1 RPC connection — confirmed 3.6.0; runtime = file offset + 0x100000 (RPC now only for small reads; SKILL drives I/O)
- [x] O-T2 Autorun — cliclick types `LOAD"PRG0:P",0` + `RUN` (sb-oracle skill)
- [x] O-T3 Program injection — write a VALID extdata file (header + HMAC-SHA1 footer; format cracked)
- [x] O-T4 Value/stdout capture — program SAVEs result to TXT; read `body[80:-20]` off disk
- [x] O-T5 ERRNUM/ERRLINE capture — `run_case.py errcase` / `|err` cases. SB has no error trapping (an error halts the program; `EXEC`/`RUN n` can't resume), so run `<stmt>`+sentinel; on halt read `ERRNUM`/`ERRLINE` in DIRECT mode. **Verified on real SB 3.6.0:** `A=SQR(-1)` → `errnum=10` (Out of range), `errline=1` — ERRNUM/ERRLINE do persist into DIRECT mode post-halt
- [x] O-T6 Graphics capture — `run_case.py grp` / `capture_grp`: program draws → `SAVE"GRPn:..."` → decode GRP off disk (28-byte PCBN header + 512×512 RGBA5551 LE) → PNG. **Verified on real SB 3.6.0** (pixel-exact). GRP pages are 512×512 buffers independent of XSCREEN mode (capture per page for both screens). Composite/sprite/BG display → `screenshot` (Ctrl+P). Goldens → `harness/corpus/golden/gfx/`
- [~] O-T7 Audio — NO deterministic emulator golden possible (SB can't render audio to disk; emulator audio is real-time/timing-dependent). Deterministic gate moves to **MML→note-events + synth params** from docs+disasm (no emulator; see M5/S-T10/S-C5). Reference-only capture built: `sb_audio.py` (Azahar `Tools>Dump Video` + ffmpeg→WAV); ffmpeg extract verified, live dump orchestration UNTESTED. **⚠ audio output accuracy is NOT end-to-end verifiable — we have no audio e2e test setup; the mechanism works as far as tested (ffmpeg extract) but the capture orchestration + any fidelity claim are practical-only/unverified. Full verification is a deferred refining layer.**
- [x] O-T8 harvest.py end-to-end — wire `run_case` into `harness/harvest`: batch spec/corpus cases → write `spec/tests` (`hw_verified`) + golden media; open PR → O-T5 (`harvest.py <stems>|--category|--all` collects inline `tests:` → batch case-lines (num/str/err mode from code+expect+return-type) → `run_case.py batch` resumable OUTFILE → folds into `spec/tests/<stem>.yaml` overlays, diffs vs inline expect (CONFIRMED/MISMATCH/NEW), prints manual git/PR steps. `--from-tsv` folds offline; `test_harvest.py` covers the pure collect/parse/fold logic in CI without Azahar. gfx/audio goldens stay on `run_case grp`/`screenshot`; live harvest + `confidence` bump are the reviewed maintainer op.)

## M0 — Scaffolding & spec pipeline ✅
- [x] M0-T1 Rust workspace + 6 crates (native + wasm32)
- [x] M0-T2 Tools into `tools/`
- [x] M0-T3 Spec skeleton + reference tables (doc-only instruction specs since DELETED — see S)
- [x] M0-T4 `sb-spec` loader + coverage + test-overlay merge
- [x] M0-T5 Harness skeleton + ported goldens + sbsave corpus
- [x] M0-T6 CI (deterministic replay only) + git

## M1 — Core VM + a real window  (gated on S; the existing lexer/AST predate the spec-first pivot — rewrite/validate, don't trust)
- [x] M1-T1 Lexer (token.rs + lexer.rs) — spec-first rewrite in fresh `crates/sb-core` (`token.rs`+`lexer.rs`); Unicode-letter identifiers (full-width/kana, NOT osb's ASCII-only), case-folded; `$`/`%`/`#` suffixes; `@label`/`#const`; `&H`/`&B` i32-wrap; `.`-leading/trailing reals + i32→Double promotion; tolerant strings; `'`/`REM` comments; two-char ops; TRUE/FALSE→1/0; SourceLoc across `:`/newlines/CRLF; 17 unit tests. Exact identifier class + leading-digit rule queued for oracle (HARVEST_QUEUE).
- [x] M1-T2 AST (ast.rs) — fresh, self-contained node types in `crates/sb-core/src/ast.rs` aligned to the M1-T1 lexer (`SourceLoc`/`TokenKind`/`Suffix`); pre-pivot ast.rs was bound to a non-existent `value.rs`/`SbString`/`TokenType` so rewritten spec-first. Expr/Stmt `{kind, loc}` nodes; typed `BinOp`/`UnOp` with `from_token` (symbolic) + `from_word` (AND/OR/XOR/MOD/DIV/NOT idents) + `rank` (getOPRank); AST-local `Lit` (decoupled from M1-T4 Value); `Name {ident, suffix}` identity; `is_lvalue`; full statement set (IF/FOR/WHILE/REPEAT/GOTO/GOSUB/ON/DEF/DIM/PRINT/INPUT/DATA/READ/RESTORE/…). 7 unit tests; Debug/Clone/PartialEq round-trip. `^` power op left out (lexer has no caret; rank queued). → M1-T1
- [x] M1-T3 Parser — recursive descent + precedence + const folding → M1-T2, S-T6
- [x] M1-T4 Value/Array completion (1–4D, refs, coercion) — `crates/sb-core/src/{value.rs,array.rs}`: `Value` enum (Void/Int i32/Real f64/Str UTF-16 + Int/Real/Str arrays + scalar `Ref`); `SbArray<T>` 1–4D row-major (natural axis order, NOT osb's reversed `type.d` — proven by hw_verified `DIM POS[3,2]:POS[2,1]`) with get/set/push/pop/shift/unshift/resize/len. Arrays are reference types (`Rc<RefCell>`, wasm-safe); scalar refs via `Cell`+`swap_cells` for OUT/SWAP. Coercion hw_verified (sb-oracle 2026-06-23): Double→Integer **truncates toward zero** (2.5→2, 4.5→4, -2.5→-2), no-suffix keeps runtime type, Int→Real widens, string↔numeric → Type mismatch (8). Array errnums hw_verified: rank mismatch → **errnum 3** (Syntax error), OOR → 31. 25 new unit tests; coercion+errnum cases folded into var.yaml/dim.yaml (hw_verified) + edges queued.
- [x] M1-T5 Bytecode + Compiler — `bytecode.rs` (flat `Op` enum + `Const`/`VarRef`/`VarType`/`Function`/`Program`) + `compiler.rs` (AST→bytecode): var resolution (global index / DEF-local bp-relative), OPTION STRICT (undeclared→errnum 15) + auto-declare + DEFINT, backpatched labels (undefined→errnum 14), DATA pool + RESTORE@label→data-index, DEF/COMMON funcs (addressed, name-dispatched), array/ref/paren-form disambiguation, osb-shaped IF/FOR/WHILE/REPEAT lowering + short-circuit &&/||. 20 unit + corpus no-panic sweep (3,329 programs, 0 panics). Builtin disambiguation deferred to M1-T7 via `Builtins` predicate; lowering edges queued. → M1-T3, M1-T4
- [x] M1-T6 VM (stack machine, 4 slots + COMMON) → M1-T5
- [x] M1-T7 Builtin registration + math/string builtins → M1-T6, S-T1, S-T2
- [x] M1-T8 Control-flow + console builtins → M1-T7, M1-T10, S-T3, S-T5
- [x] M1-T9 TinyMT RNG (RND/RNDF/RANDOMIZE) → M1-T7, S-T1
- [x] M1-T10 Console model + render → framebuffer → (M0 sb-render)
- [x] M1-T11 Headless runner `sb-run` — new `sb-platform-native` crate (`src/bin/sb-run.rs`): loads a `.sb3` (plain UTF-8 source), runs it through sb-core (`parse → compile_with(StdBuiltins) → Vm::run`) headless, dumps `console_text()` to stdout; on a SmileBASIC error prints `ERRNUM`/`ERRLINE` to stderr. Exit codes: 0 success/STOP, 1 SB error (parse errnum 3 / compile / runtime e.g. SQR(-1)→10), 2 usage/unreadable-file. This is the `target/debug/sb-run` that `harness/diff/replay.py` shells out to. 4 bin tests (fizzbuzz fixture, console text, runtime/parse errnum). → M1-T8
- [x] M1-T12 Window (native winit + wasm canvas) — `crates/sb-platform-native/src/bin/sb.rs` (new `sb` bin): runs a `.sb3` through the same `parse→compile_with(StdBuiltins)→Vm::run` pipeline as `sb-run`, renders `vm.console()` into an `sb_render::Framebuffer` (opaque-black backdrop so transparent-bg console cells blit), and blits it to a winit 0.30 + softbuffer 0.4 window (nearest-neighbour scale-to-fit, 2× default, redraw-on-resize; partial console still shown on a halt). winit/softbuffer are target-gated `not(wasm32)` and the whole bin is an empty `main` on wasm32, so `--target wasm32-unknown-unknown` stays clean. New `sb-platform-wasm` crate (cdylib+rlib): `render_program(src)→Framebuffer` (shared, native-testable) + wasm-only `#[wasm_bindgen] run_program(canvas_id, src)` that blits the RGBA8888 framebuffer to a `<canvas>` via `put_image_data` (web-sys gated to wasm32). 3 new tests (native `sb`: lit-pixels + error-still-renders; wasm: lit-pixels). → M1-T10
- [x] M1-T13 Error model + ERRNUM/ERRLINE — new `crates/sb-core/src/sysvars.rs` (`ErrSysvar` enum: the three read-only error-state sysvars). VM tracks `errnum`/`errline`/`errprg` (boot/fresh-run = 0); a halting `VmError::Sb` stamps them in `run()` so they're readable post-halt as the DIRECT-mode residue (accessors `errnum()`/`errline()`/`errprg()`; `ERRPRG`=0 in single-slot M1, multi-slot → M6). Compiler resolves a bare-name read of `ERRNUM`/`ERRLINE`/`ERRPRG` to new `Op::PushSysvar` (reserved — resolves before user vars/builtins); assigning to one is a compile-time Syntax error (errnum 3) per `sysvars.yaml writable=false`. 6 new tests (errnum 8/7/31/4/10 cases, ERRLINE/ERRPRG persistence, clean-run reads 0, read-only rejection) + 2 sysvars unit tests; 3,329-program corpus sweep still 0 panics. → M1-T6, S-T14
- [x] M1-T14 Conformance wiring (run spec/tests + corpus; ASSERT__; otya_test) → M1-T11

## M2 — Graphics  (gated on S-T7)
- [x] M2-T1 GRP page model → S-T7
- [x] M2-T2 Drawing primitives → M2-T1
- [x] M2-T3 Bitmap ops → M2-T1
- [x] M2-T4 Compositor → M2-T2, M2-T3
- [x] M2-T5 Golden PNG harvest + pixel-diff → M2-T4, O-T6

## M3 — Sprites & BG  (gated on S-T8, S-T9)
- [x] M3-T1 Sprite core → S-T8, M2-T4
- [x] M3-T2 Animation/link/vars → M3-T1
- [x] M3-T3 Collision → M3-T1
- [x] M3-T4 BG core → S-T9, M2-T4
- [x] M3-T5 BG extras → M3-T4
- [x] M3-T6 Composite + golden diffs → M3-T2, M3-T3, M3-T5, O-T6

## M4 — Input & timing  (gated on S-T11)
- [x] M4-T1 Buttons/sticks → S-T11
- [x] M4-T2 Touch/keyboard → S-T11
- [x] M4-T3 Frame timing (VSYNC/WAIT/MAINCNT) → S-T4
- [x] M4-T4 Display config → S-T11
- [x] M4-T5 Host input mapping → M4-T1, M4-T2

## M5 — Audio (MML)  (gated on S-T10)
> **⚠ Audio output accuracy can't be e2e-verified — no audio test setup (see O-T7).** MML
> parsing + synth params (M5-T1..T4) ARE verifiable deterministically (MML→note-events vs
> docs/disasm); the *rendered sound's* fidelity is practical-only (ear-check / loose spectral)
> until a real audio e2e harness exists. Treat audio-fidelity claims as unverified; full
> verification is a deferred refining layer.
- [x] M5-T1 MML parser → S-C5  (parse-to-events: deterministically verifiable) — new `sb-audio` crate; `mml.rs` parses an MML string → per-channel `Vec<Event>` timeline (channels, tempo/length/gate/ties/portamento, pitch/octave/key, volume/pan/envelope, instruments, detune/LFOs/modulation, finite-unrolled `[ ]N` repeats + endless-loop markers, `$0`–`$7` vars, case-sensitive `{macro}`s); malformed → errnum 47 with caret offset. 35 unit tests + a 550-string corpus sweep (98.4% of complete real BGM* literals parse). Corpus-surfaced forms folded in as community/oracle-pending (`(N`/`)N` volume steps, dotted `L<n>.`, leading accidentals, case-sensitive labels) — spec S-C5 + HARVEST_QUEUE updated.
- [x] M5-T2 Synth engine → M5-T1  (⚠ output fidelity not e2e-verifiable; param tables are) — new `synth.rs`+`instruments.rs` render a parsed `Song` → interleaved stereo PCM16. **Signal path grounded on the real 3DS DSP** via citra/azahar `audio_core`: native rate 32728 Hz, 160-sample frames, per-voice fractional resample with the DSP's Q24 linear interpolation + saturated delta (`interpolate.cpp` `Linear`). Instruments = single-cycle wavetables (Saw/Pulse/Triangle/Sine/Noise) resampled like the hardware sample ROM; ADSR (`@E`), gate `Q`, per-note velocity/`V` volume, equal-power pan `P`, `@D` detune, portamento `_`, `@MON`-gated vibrato/tremolo/autopan LFOs, 16-channel additive mix with saturating clamp. Timing per S-C5 (48 ticks/quarter, `samples/tick=32728·60/(T·48)`). Fully **deterministic** (integer/`f32` math, seeded-LCG noise) — same MML → byte-identical PCM. 25 new tests (timing/tempo, pitch via zero-crossings, octave/detune, pan, gate staccato, mix, endless-loop frame-budget fill, interp endpoints, ADSR). Per **O-T7** there is no emulator audio golden, so output *fidelity* (real instrument ROM, exact envelope/LFO/`@V` curves, drum samples) is the **deferred refining layer** — queued in `HARVEST_QUEUE.md`.
- [x] M5-T3 BGM commands → M5-T2, S-T10
- [x] M5-T4 SFX/voice → M5-T2, S-T10 — `BEEP` (preset SFX: sound 0..133|224..255|256..383, freq/vol/pan ranges, empty-comma skip, pan→`pan*2-128` remap), `TALK`/`TALKCHK`/`TALKSTOP` (speech transport: idle→0, playing→1, shape errnum 4), `EFCSET`/`EFCON`/`EFCOFF`/`EFCWET` (music effector over new `sb-audio::effects::EffectState` — preset 0..3 / 7-arg raw reverb, on/off flag, per-source wet 0..127 with TALK≥64 ON, errnum 3/10 guards), `WAVSET`/`WAVSETA` (user MML instruments @224..255 over `effects::UserInstrument` — hex-string decode 16/32/64/128/256/512 samples / numeric-array slice, ADSR 0..127, ref-pitch default 69, errnum 4/8/10). Routes over `AudioState` in `sound.rs`; new `effects.rs` holds the pure models. All S-T10b/c/d inline spec cases fold into the conformance gate (`IN_SCOPE_SOUND`), plus 8 sound.rs unit + 6 VM e2e tests. (Audible output unverifiable — O-T7.) Fixed two fixtures: wavseta test code `DIM SMP(16)`→`DIM SMP[16]` (canonical bracket array-decl our parser accepts), talkchk `"0\n"`→`"0"` (console_text scrape convention). Queued: function-as-statement errnum-3 rejection, WAVSET `[]` repeat-group form.
- [x] M5-T5 Audio backend → M5-T2 — new device-independent streaming core `sb-audio::stream` (`PcmRing` ring buffer: silence+counted-underrun on starvation; stateful `StereoResampler`: linear, phase-continuous across chunk seams, streaming==one-shot proven) — 10 unit tests, always built+tested. Live backends: `sb-platform-native::audio` (cpal `AudioBackend`/`play_blocking`, F32/I16/U16 + mono/stereo/surround spreading, off-by-default `audio` feature so headless-ubuntu CI w/o ALSA stays green) + `sb-play` demo bin; `sb-platform-wasm` WebAudio (`WebAudio`/`#[wasm_bindgen] play_mml`, planar AudioBuffer @32728→browser-resample), wasm-gated so the wasm build covers it. Audible output unverifiable (O-T7); the deterministic gate covers the pure ring/resampler.
- [ ] M5-T6 Golden WAV harvest + diff → M5-T3, M5-T4, O-T7  (⚠ NOT a deterministic golden — reference/loose-spectral only; deferred refining layer)

## M6 — Files, projects, system, faithful stubs  (gated on S-T12)
- [x] M6-T1 Storage abstraction → S-T12 — new `sb-core::storage` (wasm-safe, I/O-free): the `Storage` trait (`projects`/`list`/`read`/`write`/`delete`/`rename`/`exists`, keyed `(project, Folder{Txt|Dat}, in-SB name)`, sorted/deterministic), the logical resource model (`parse_resource` splits `"TYPE:NAME"` → `ResourceKind` {Program 0-3 / Graphic 0-5 / GraphicFont / Text / Data} with the disassembled `SAVE`-handler errnum map: unknown type → 4, index past family → 10; `FilesFilter` for FILES), `MemStorage` in-memory impl + deterministic `serialize`/`deserialize`, and an `extdata` codec (`wrap`/`unwrap` of the 80-byte-header + body + 20-byte-HMAC-SHA1-footer container, dependency-free SHA-1/HMAC). HMAC footer **cross-checked byte-for-byte** against the oracle's `sb_extdata.py` golden (TXT `PRINT 1` → `6d7b94ed…`), plus SHA1/HMAC RFC test vectors. Platform impls: `sb-platform-native::storage::FsStorage` (real `<root>/<project>/{TXT,DAT}/<name>` tree matching the corpus layout) + `sb-platform-wasm::storage::IdbStorage` (in-memory mirror persisted to IndexedDB as one serialized blob; wasm32-gated). 21 sb-core + 4 native storage tests; full gate green incl. wasm build.
- [x] M6-T2 File commands → M6-T1 — new `crates/sb-core/src/builtins/files.rs` + VM routing (`call_files`): `SAVE`/`LOAD`/`FILES`/`DELETE`/`RENAME`/`CHKFILE`/`PROJECT` over a VM-owned `Storage` (defaults to in-memory `MemStorage`; `Vm::set_storage` injects a real FS impl) + `current_project`/`current_slot`. `SAVE "TXT:",str` / `LOAD("TXT:")`/`OUT`/function forms round-trip UTF-8 text; `SAVE "DAT:",arr` / `LOAD "DAT:",arr` round-trip numeric arrays via a self-describing `"SBDA"` body codec (Int/Real, 1-D auto-extend; real PCBN byte layout queued O-T3, foreign body→errnum 35); `FILES ["filter",]arr$` fills a sorted name array (TXT:/DAT://-projects/NAME-project filters) or lists to console; `DELETE`/`RENAME`/`CHKFILE` over the resource→`(folder,name)` map; `PROJECT OUT p$` reads the current project, set form in a program → errnum 44, `PROJECT=v` stays a variable. All 7 specs' inline arg-shape (3/4) / type (8) / DIRECT-only (44) guards + the `PROJECT=v` variable form fold into the gate (`IN_SCOPE_FILES`); 11 round-trip/listing cases in `harness/corpus/cases/files.yaml` + 4 codec unit tests. Queued (O-T3/M6-T4/M6-T6): real PCBN tagging, program/GRP payload plumbing, oracle-confirm 46/35, multi-slot bare-name routing.
- [x] M6-T3 System variables → S-T14 — unified `sysvars::Sysvar` (21 names; `TRUE`/`FALSE` stay lexer literals, `HARDWARE` the M4-T4 builtin) replacing the fragmented `ErrSysvar`+`PushMaincnt`. Compiler resolves a bare sysvar ahead of user vars → `Op::PushSysvar`; a writable one (`TABSTEP`/`SYSBEEP`) assigns via new `Op::StoreSysvar`, every read-only one → Syntax error (errnum 3). VM `read_sysvar`/`write_sysvar`: `VERSION`=&H03060000, `MAINCNT` from the frame clock, `ERR*` from error state, `CSRX`/`CSRY` from the live console cursor (`CSRZ`=0 flat grid), `TABSTEP`/`SYSBEEP` round-trip (boot 4/1) + `SYSBEEP` exposed via `Vm::sysbeep`, `FREEMEM` a faithful constant, `RESULT`/`CALLIDX`/`PRGSLOT`/mic/MP stubs. `DATE$`/`TIME$` over a new injectable deterministic `clock::WallClock` (`Vm::set_wall_clock`; epoch 2000/01/01). **hw_verified (sb-oracle 2026-06-23):** offline `MPHOST`/`MPLOCAL`=-1, `RESULT`=1 (boot TRUE), `MPCOUNT`/`MICPOS`/`MICSIZE`/`CSRZ`=0, `FREEMEM`≈8314876 (near-empty snapshot) — folded into `sysvars.yaml` goldens. 18 new lib tests (writability, read-only rejection of all 12, VERSION, deterministic DATE$/TIME$, TABSTEP tab effect, cursor tracking, stub values); queued: FREEMEM allocator model, PRGSLOT/RESULT/TABSTEP-range/SYSBEEP-truthiness oracle confirms.
- [x] M6-T4 Source-edit (PRG*) → M6-T1, S-T12 — new `crates/sb-core/src/builtins/prg.rs` (pure per-slot source model: `PrgSlot{name,lines}` + LF terminator/separator splitters, `char_count`/`free_count`) + VM routing (`call_prg`). The VM owns four program-slot sources (`prg_slots`) + an active edit target (`prg_edit`, `None`=cold). `PRGEDIT slot[,line]` selects the target (arg-count→4, slot 0-3→10, running-slot→4, `line=-1`=last); `PRGGET$`/`PRGSET`/`PRGINS str[,flag]`/`PRGDEL [count]` read/replace/insert/delete the current line (CHR$(10) splits multi-line, negative count deletes all remaining, count-0→10, cold-state→38 checked before arg-count); `PRGNAME$([slot])`/`PRGSIZE([slot[,type]])` report a slot's file name / line-char-free counts. New `Vm::set_slot_source` seeds a slot (host/test). All 7 specs' hw_verified arg-shape (4) / range (10) / cold-state (38) guards fold into the conformance gate (`IN_SCOPE_PRG`); 11 VM round-trip/guard + 3 prg.rs unit tests. Content/counts/capacity oracle-pending (queued in HARVEST_QUEUE.md).
- [x] M6-T5 Misc + limitation stubs → S-T12 — new `crates/sb-core/src/builtins/device.rs` (pure logic + `DeviceState` XON/XOFF feature flags) + VM routing (`call_device`) for the faithful "limitation stub" family: `XON`/`XOFF` (parser keyword form → synthetic feature code; EXPAD sets RESULT TRUE), the microphone (`MICSTART`/`MICSTOP`/`MICDATA`/`MICSAVE`), the motion sensors (`GYROA`/`GYROV`/`GYROSYNC`/`ACCEL`), wireless multiplayer (`MPSTART`/`MPEND`/`MPSET`/`MPSTAT`/`MPSEND`/`MPRECV`/`MPGET`/`MPNAME$`), and `DIALOG`. None of the hardware exists headless, so each reproduces its *observable* contract — the disassembled arg-shape (4) / range (10) / type (8) / syntax (3) guards + the hw_verified XON-MIC (36) / XON-MOTION (37) availability errors — over neutral stub outputs (MICDATA→0, sensors→0.0, DIALOG→RESULT 0/-1, MP offline: MPSTART RESULT 0, MPSTAT()→0, MPRECV→SID -1, peer reads→errnum 10). MP-restriction flag treated as 0 (reachable in DIRECT/program mode, per the oracle). All 12 specs' inline guard cases fold into the gate (`IN_SCOPE_DEVICE`); 6 device.rs unit + 5 VM e2e tests. (FONTDEF already lands in M6-T2/console; SB3 has no CLIPBOARD instruction. DIRECT-only→43 gating is tied to the unimplemented DIRECT-mode keyword commands — RUN/LIST/NEW/CLEAR/CONT, S-T12e — and PROJECT already covers the program→44 direction (M6-T2).) Live device output + interactive DIALOG/MP-session values oracle-pending (HARVEST_QUEUE.md).
- [x] M6-T6 Multi-slot semantics → M6-T1 — 4 slots + shared COMMON DEF, EXEC/USE/CALL, cross-slot scoping, CALLIDX in SPFUNC/BGFUNC callbacks. CALL-by-name (`Op::CallDynamic`), USE/EXEC hw_verified slot/resource error model, cross-slot COMMON DEF dispatch (`activate_slot`/per-slot program registry), EXEC numeric+string control transfer (load-from-storage, running-slot restart `EXEC 0`, bare-name default-slot, slot-0 registry edge, `END`-returns-to-launcher LIFO), cross-slot variable scoping (defining-slot binding, osb-structural), CALL SPRITE/BG callback dispatch + CALLIDX, and (final piece) bare-name `USE "file"`→running slot→errnum 4 (hw_verified) — no `VmError::Unsupported` arm remains in USE/EXEC. Residue = oracle-pending refinements (resume-state granularity, ≥2-slot scoping confirm, callback quirks), queued in HARVEST_QUEUE.md.

## M7 — Hardening
- [x] M7-T1 Fuzzing campaign → O-T8 — fleshed `harness/fuzz/generator.py` into a seeded, spec-signature-driven generator (reads all 248 `spec/instructions/*.yaml` signatures; well-typed expr/stmt grammar; "safe" profile = math/string/bit + operators + bounded FOR, guaranteed-terminating [loop bodies can't reassign the counter] → VM-runnable; "broad" profile adds arbitrary scalar-arg commands → parse/compile-only). Wired `harness/diff/run.py` into a crash-classifying differential runner over `sb-run`. **Documented campaign** (`ralph-logs/fuzz-campaign-20260623.md`, 1000+1000 seeds) found + **fixed 3 genuine host panics**: GTRI i32 edge-function overflow (`raster.rs` → i128 geometry), GCOPY span/`abs()`/`w*h` overflow (`bitmap.rs` → i64 span + offset-preserving page-intersection clamp + overflow-proof `read_region`), VAL `&H`/`&B` prefix byte-slice mid-codepoint on a full-width glyph (`string.rs` → `get(..n)`). Each repro A/B-verified (panics pre-fix, passes post-fix). Promoted into `harness/corpus/fuzz/` (regressions [run] + safe [run] + broad [compile]) replayed deterministically by `crates/sb-core/tests/fuzz_corpus.rs`. Oracle 3-way VALUE differential + extreme-coord GTRI/GCOPY clamp-vs-errnum confirmation are the offline maintainer step (queued, per O-T8 convention).
- [x] M7-T2 hw_verified push → O-T8 — ≥80% target reached (80.2% hw_verified, 199/248) via iterative oracle harvest of every value/errnum-deterministic surface; remaining `disassembled` set is the structural media/hardware ceiling (O-T6/O-T7 + the input/mic/motion/wireless stub family)
- [x] M7-T3 Exact error strings → O-T5 — new `crates/sb-core/src/error.rs`: the canonical `errnum → message` authority (`ERROR_NAMES[0..=55]` + `error_message`), reproduced **byte-for-byte from the binary** by dereferencing all 56 pointers at `*[0x3054f8]` into the `.rodata` ASCII pool `[0x2e965c,0x2e9ac0)` (not re-spelled from docs — catches the binary's case: 43 "Can't use from **direct** mode", 41 "String is too long" ≠ docs "String too long"). `error_message` mirrors the formatter `FUN_001e94a8` exactly: real display is errnum∈[1,55]→`table[errnum]`, while errnum 0 (cleared) and ≥56 take the `"Internal Error"` fallback (`adrcs r9,0x1e9588`), so pool[0] "No Error" is never shown; the binary then optionally appends " (detail)" + a location string. `message()` accessors added to `VmError`/`ParseError`/`CompileError`/`RuntimeError` (the four errnum carriers) surface the SB string distinct from their diagnostic detail. errnum assignments verified: VM's errnum consts (4/5/6/7/8/10/13/14/16/30/31/38/46) all match the byte-for-byte table; the hw_verified 4/7/8/10 from S-T14a stand. 3 golden tests (byte-for-byte pool, display semantics, out-of-band fallback); `errors.yaml` source enriched with the full-dereference + formatter range/fallback detail.
- [x] M7-T4 Float formatting (STR$/PRINT) → S-T1 — reverse-engineered SB's two distinct double→string formatters from the disassembly: **STR$** = C `%g`/6-sig (handler @0x1eb2a8, fmt "%g" @0x1eb4a8 via sprintf @0x1ec1f0) — `builtins::format_g` verified exact against a 2000-case bit-exact `%.6g` sweep + oracle; **PRINT** = C `%.8f` then trailing-zero/bare-dot trim (handler @0x180a50, fmt "%.8f" @0x180b0c + back-scan trim loop @0x180a8c), NEVER exponential — was wrongly sharing STR$'s `%g`; new `format_print_number`/`format_fixed8` routed through `format_print_item`. Both preserve signed zero (STR$(-0.0)/PRINT -0.0 → "-0"). hw_verified PRINT values harvested via console read-back (PRINT 12345678.0="12345678" vs STR$="1.23457e+07"; PRINT 0.00001="0.00001"; PRINT 1/3="0.33333333"). Fixed 24 math-spec test cases that conflated PRINT output with the STR$ value; broad %g + %.8f conformance tables in str.yaml/print.yaml + unit tests.
- [ ] M7-T5 Overflow/precision + perf → M7-T4
