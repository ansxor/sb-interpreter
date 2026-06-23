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
| M1 | Core VM + a real window | `prd/M1.md` | ⬜ gated on S (pre-pivot lexer/AST exist — redo) |
| M2 | Graphics (GRP + compositor) | `prd/M2.md` | ⬜ gated on S |
| M3 | Sprites & BG | `prd/M3.md` | ⬜ gated on S |
| M4 | Input & timing | `prd/M4.md` | ⬜ gated on S |
| M5 | Audio (MML) | `prd/M5.md` | ⬜ gated on S |
| M6 | Files, projects, system, stubs | `prd/M6.md` | ⬜ gated on S |
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
- [ ] S-T10a BGM playback — BGMPLAY · BGMSTOP · BGMCHK · BGMVOL · BGMVAR
- [ ] S-T10b BGM setup — BGMSET · BGMSETD · BGMCLEAR · BEEP
- [ ] S-T10c Effects — EFCON · EFCOFF · EFCSET · EFCWET
- [ ] S-T10d Voice & wave — TALK · TALKCHK · TALKSTOP · WAVSET · WAVSETA

#### S-T11 Various input + Screen control (20) → S-T0
- [ ] S-T11a Buttons & sticks — BUTTON · BREPEAT · STICK · STICKEX
- [ ] S-T11b Touch & motion — TOUCH · ACCEL · GYROA · GYROV · GYROSYNC
- [ ] S-T11c Microphone — MICSTART · MICSTOP · MICDATA · MICSAVE
- [ ] S-T11d Screen control — ACLS · BACKCOLOR · DISPLAY · VISIBLE · XSCREEN
- [ ] S-T11e Fade — FADE · FADECHK

#### S-T12 Files + Source-manip + DIRECT-mode (22) → S-T0
- [ ] S-T12a File I/O — LOAD · SAVE · FILES · DELETE
- [ ] S-T12b File management — CHKFILE · RENAME · USE · EXEC
- [ ] S-T12c Source read — PRGGET$ · PRGNAME$ · PRGSIZE
- [ ] S-T12d Source edit — PRGSET · PRGINS · PRGDEL · PRGEDIT
- [ ] S-T12e DIRECT-mode — RUN · CONT · NEW · CLEAR · LIST · BACKTRACE · PROJECT

#### S-T13 Wireless (8) → S-T0
- [ ] S-T13a Session — MPSTART · MPEND · MPSET · MPSTAT
- [ ] S-T13b Messaging — MPSEND · MPRECV · MPGET · MPNAME$

#### S-T14 Verify reference tables (vs disassembly + oracle) → O-T4
- [ ] S-T14a Error table — `spec/reference/errors.yaml` vs disasm error strings (@≈0x1E965C) + oracle → O-T5
- [ ] S-T14b System variables — `spec/reference/sysvars.yaml` vs disasm sysvar addrs + oracle
- [ ] S-T14c Built-in constants — `spec/reference/constants.yaml` vs disasm constant table + oracle

### S-C — Concept specs (architecture/models; Markdown in `spec/concepts/`, see prd/specs.md)
- [ ] S-C1 execution-model — lexer/parser/compiler/VM, 4 slots + COMMON, frame layout · governs M1
- [~] S-C2 screen-and-color-model — layers/Z/RGBA5551 (exemplar drafted; confirm vs oracle) · governs M2, O-T6
- [ ] S-C3 sprite-bg-model — attributes/animation/collision/tilemaps · governs M3
- [ ] S-C4 frame-and-timing-model — VSYNC/WAIT/MAINCNT, 60 fps · governs M4
- [ ] S-C5 mml-grammar — the full MML language · governs M5
- [ ] S-C6 file-and-extdata-format — projects/resources/extdata layout · governs M6, O-T3
- [ ] S-C7 error-model — errnum/ERRLINE, halt/CONT semantics · governs M1, O-T5

## O — Oracle engine — implemented as the `.claude/skills/sb-oracle/` skill (Azahar + cliclick + extdata)
- [x] O-T1 RPC connection — confirmed 3.6.0; runtime = file offset + 0x100000 (RPC now only for small reads; SKILL drives I/O)
- [x] O-T2 Autorun — cliclick types `LOAD"PRG0:P",0` + `RUN` (sb-oracle skill)
- [x] O-T3 Program injection — write a VALID extdata file (header + HMAC-SHA1 footer; format cracked)
- [x] O-T4 Value/stdout capture — program SAVEs result to TXT; read `body[80:-20]` off disk
- [x] O-T5 ERRNUM/ERRLINE capture — `run_case.py errcase` / `|err` cases. SB has no error trapping (an error halts the program; `EXEC`/`RUN n` can't resume), so run `<stmt>`+sentinel; on halt read `ERRNUM`/`ERRLINE` in DIRECT mode. **Verified on real SB 3.6.0:** `A=SQR(-1)` → `errnum=10` (Out of range), `errline=1` — ERRNUM/ERRLINE do persist into DIRECT mode post-halt
- [x] O-T6 Graphics capture — `run_case.py grp` / `capture_grp`: program draws → `SAVE"GRPn:..."` → decode GRP off disk (28-byte PCBN header + 512×512 RGBA5551 LE) → PNG. **Verified on real SB 3.6.0** (pixel-exact). GRP pages are 512×512 buffers independent of XSCREEN mode (capture per page for both screens). Composite/sprite/BG display → `screenshot` (Ctrl+P). Goldens → `harness/corpus/golden/gfx/`
- [~] O-T7 Audio — NO deterministic emulator golden possible (SB can't render audio to disk; emulator audio is real-time/timing-dependent). Deterministic gate moves to **MML→note-events + synth params** from docs+disasm (no emulator; see M5/S-T10/S-C5). Reference-only capture built: `sb_audio.py` (Azahar `Tools>Dump Video` + ffmpeg→WAV); ffmpeg extract verified, live dump orchestration UNTESTED. **⚠ audio output accuracy is NOT end-to-end verifiable — we have no audio e2e test setup; the mechanism works as far as tested (ffmpeg extract) but the capture orchestration + any fidelity claim are practical-only/unverified. Full verification is a deferred refining layer.**
- [ ] O-T8 harvest.py end-to-end — wire `run_case` into `harness/harvest`: batch spec/corpus cases → write `spec/tests` (`hw_verified`) + golden media; open PR → O-T5

## M0 — Scaffolding & spec pipeline ✅
- [x] M0-T1 Rust workspace + 6 crates (native + wasm32)
- [x] M0-T2 Tools into `tools/`
- [x] M0-T3 Spec skeleton + reference tables (doc-only instruction specs since DELETED — see S)
- [x] M0-T4 `sb-spec` loader + coverage + test-overlay merge
- [x] M0-T5 Harness skeleton + ported goldens + sbsave corpus
- [x] M0-T6 CI (deterministic replay only) + git

## M1 — Core VM + a real window  (gated on S; the existing lexer/AST predate the spec-first pivot — rewrite/validate, don't trust)
- [ ] M1-T1 Lexer (token.rs + lexer.rs) — ⚠ existing code is an osb-port (ASCII-only idents); redo spec-first, verify identifier rules vs disassembly/oracle
- [ ] M1-T2 AST (ast.rs) — exists from the pre-pivot attempt; revalidate against the parser + specs → M1-T1
- [ ] M1-T3 Parser — recursive descent + precedence + const folding → M1-T2, S-T6
- [ ] M1-T4 Value/Array completion (1–4D, refs, coercion)
- [ ] M1-T5 Bytecode + Compiler → M1-T3, M1-T4
- [ ] M1-T6 VM (stack machine, 4 slots + COMMON) → M1-T5
- [ ] M1-T7 Builtin registration + math/string builtins → M1-T6, S-T1, S-T2
- [ ] M1-T8 Control-flow + console builtins → M1-T7, M1-T10, S-T3, S-T5
- [ ] M1-T9 TinyMT RNG (RND/RNDF/RANDOMIZE) → M1-T7, S-T1
- [ ] M1-T10 Console model + render → framebuffer → (M0 sb-render)
- [ ] M1-T11 Headless runner `sb-run` → M1-T8
- [ ] M1-T12 Window (native winit + wasm canvas) → M1-T10
- [ ] M1-T13 Error model + ERRNUM/ERRLINE → M1-T6, S-T14
- [ ] M1-T14 Conformance wiring (run spec/tests + corpus; ASSERT__; otya_test) → M1-T11

## M2 — Graphics  (gated on S-T7)
- [ ] M2-T1 GRP page model → S-T7
- [ ] M2-T2 Drawing primitives → M2-T1
- [ ] M2-T3 Bitmap ops → M2-T1
- [ ] M2-T4 Compositor → M2-T2, M2-T3
- [ ] M2-T5 Golden PNG harvest + pixel-diff → M2-T4, O-T6

## M3 — Sprites & BG  (gated on S-T8, S-T9)
- [ ] M3-T1 Sprite core → S-T8, M2-T4
- [ ] M3-T2 Animation/link/vars → M3-T1
- [ ] M3-T3 Collision → M3-T1
- [ ] M3-T4 BG core → S-T9, M2-T4
- [ ] M3-T5 BG extras → M3-T4
- [ ] M3-T6 Composite + golden diffs → M3-T2, M3-T3, M3-T5, O-T6

## M4 — Input & timing  (gated on S-T11)
- [ ] M4-T1 Buttons/sticks → S-T11
- [ ] M4-T2 Touch/keyboard → S-T11
- [ ] M4-T3 Frame timing (VSYNC/WAIT/MAINCNT) → S-T4
- [ ] M4-T4 Display config → S-T11
- [ ] M4-T5 Host input mapping → M4-T1, M4-T2

## M5 — Audio (MML)  (gated on S-T10)
> **⚠ Audio output accuracy can't be e2e-verified — no audio test setup (see O-T7).** MML
> parsing + synth params (M5-T1..T4) ARE verifiable deterministically (MML→note-events vs
> docs/disasm); the *rendered sound's* fidelity is practical-only (ear-check / loose spectral)
> until a real audio e2e harness exists. Treat audio-fidelity claims as unverified; full
> verification is a deferred refining layer.
- [ ] M5-T1 MML parser → S-C5  (parse-to-events: deterministically verifiable)
- [ ] M5-T2 Synth engine → M5-T1  (⚠ output fidelity not e2e-verifiable; param tables are)
- [ ] M5-T3 BGM commands → M5-T2, S-T10
- [ ] M5-T4 SFX/voice → M5-T2, S-T10
- [ ] M5-T5 Audio backend → M5-T2
- [ ] M5-T6 Golden WAV harvest + diff → M5-T3, M5-T4, O-T7  (⚠ NOT a deterministic golden — reference/loose-spectral only; deferred refining layer)

## M6 — Files, projects, system, faithful stubs  (gated on S-T12)
- [ ] M6-T1 Storage abstraction → S-T12
- [ ] M6-T2 File commands → M6-T1
- [ ] M6-T3 System variables → S-T14
- [ ] M6-T4 Source-edit (PRG*) → M6-T1, S-T12
- [ ] M6-T5 Misc + limitation stubs → S-T12
- [ ] M6-T6 Multi-slot semantics → M6-T1

## M7 — Hardening
- [ ] M7-T1 Fuzzing campaign → O-T8
- [ ] M7-T2 hw_verified push → O-T8
- [ ] M7-T3 Exact error strings → O-T5
- [ ] M7-T4 Float formatting (STR$) → S-T1
- [ ] M7-T5 Overflow/precision + perf → M7-T4
