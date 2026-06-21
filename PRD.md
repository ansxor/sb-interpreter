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
| **O** | **Oracle harvest engine** | `prd/oracle.md` | 🔥 active (connected) |
| M1 | Core VM + a real window | `prd/M1.md` | ◐ lexer/AST done; gated on S |
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

- [ ] S-T0 Spec schema v2 + authoring guide — define the richer spec shape (typed sigs, ranges, errors, cases) and the 4-source process; update `spec/SCHEMA.md`; `sb-spec` structs to match
- [ ] S-T1 Mathematics (27) — author specs + cases → S-T0
- [ ] S-T2 Strings (12) → S-T0
- [ ] S-T3 Control + Advanced control (22+5) → S-T0
- [ ] S-T4 Variables/arrays + Data-ops (13+14) → S-T0
- [ ] S-T5 Console I/O (12) → S-T0
- [ ] S-T6 Bit-ops + operators (5) → S-T0
- [ ] S-T7 Graphics (19) → S-T0
- [ ] S-T8 Sprites (27) → S-T0
- [ ] S-T9 BG (24) → S-T0
- [ ] S-T10 Sound + MML reference (18) → S-T0
- [ ] S-T11 Various input + Screen control (13+7) → S-T0
- [ ] S-T12 Files + Source-manip + DIRECT-mode (8+7+7) → S-T0
- [ ] S-T13 Wireless (8) → S-T0
- [ ] S-T14 Verify reference tables (errors/sysvars/constants) vs disassembly + oracle → O-T4, O-T5

## O — Oracle harvest engine (real SmileBASIC 3.6.0 in Azahar)
- [x] O-T1 RPC connection — Azahar `--install` + boot; read guest memory (banner @0x2E9AE0 confirms 3.6.0; runtime = file offset + 0x100000)
- [ ] O-T2 Autorun — drive SB to RUN a program deterministically (TAS movie record/play `-r`/`-p`, and/or RPC trigger) → O-T1
- [ ] O-T3 Program injection — get a test program into SB (extdata file format, or RPC `write_memory` into a slot) → O-T1
- [ ] O-T4 stdout capture — read the console grid from guest memory (RE its address) or CHKCHR scrape → O-T1
- [ ] O-T5 ERRNUM/ERRLINE capture — RE the sysvar addresses; read after a halt → O-T1
- [ ] O-T6 Framebuffer capture — `--dump-video` and/or RE the framebuffer address; decode to RGBA → O-T1
- [ ] O-T7 Audio capture — emulator audio dump → O-T1
- [ ] O-T8 harvest.py end-to-end — run case → capture → write `spec/tests/<id>.yaml` (`hw_verified`) + golden media → O-T2, O-T3, O-T4, O-T5

## M0 — Scaffolding & spec pipeline ✅
- [x] M0-T1 Rust workspace + 6 crates (native + wasm32)
- [x] M0-T2 Tools into `tools/`
- [x] M0-T3 Spec skeleton + reference tables (doc-only instruction specs since DELETED — see S)
- [x] M0-T4 `sb-spec` loader + coverage + test-overlay merge
- [x] M0-T5 Harness skeleton + ported goldens + sbsave corpus
- [x] M0-T6 CI (deterministic replay only) + git

## M1 — Core VM + a real window  (gated on S-T1..S-T6 for the parts it touches)
- [x] M1-T1 Lexer (token.rs + lexer.rs) — ⚠ revisit full-width identifiers (HARVEST_QUEUE)
- [x] M1-T2 AST (ast.rs)
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
- [ ] M2-T1 GRP page model · [ ] M2-T2 Drawing primitives · [ ] M2-T3 Bitmap ops · [ ] M2-T4 Compositor · [ ] M2-T5 Golden PNG harvest + pixel-diff → O-T6

## M3 — Sprites & BG  (gated on S-T8, S-T9)
- [ ] M3-T1 Sprite core · [ ] M3-T2 Animation/link/vars · [ ] M3-T3 Collision · [ ] M3-T4 BG core · [ ] M3-T5 BG extras · [ ] M3-T6 Composite + golden diffs

## M4 — Input & timing  (gated on S-T11)
- [ ] M4-T1 Buttons/sticks · [ ] M4-T2 Touch/keyboard · [ ] M4-T3 Frame timing (VSYNC/WAIT/MAINCNT) · [ ] M4-T4 Display config · [ ] M4-T5 Host input mapping

## M5 — Audio (MML)  (gated on S-T10)
- [ ] M5-T1 MML parser · [ ] M5-T2 Synth engine · [ ] M5-T3 BGM commands · [ ] M5-T4 SFX/voice · [ ] M5-T5 Audio backend · [ ] M5-T6 Golden WAV harvest + diff → O-T7

## M6 — Files, projects, system, faithful stubs  (gated on S-T12)
- [ ] M6-T1 Storage abstraction · [ ] M6-T2 File commands · [ ] M6-T3 System variables · [ ] M6-T4 Source-edit (PRG*) · [ ] M6-T5 Misc + limitation stubs · [ ] M6-T6 Multi-slot semantics

## M7 — Hardening
- [ ] M7-T1 Fuzzing campaign → O-T8 · [ ] M7-T2 hw_verified push → O-T8 · [ ] M7-T3 Exact error strings → O-T5 · [ ] M7-T4 Float formatting (STR$) · [ ] M7-T5 Overflow/precision + perf
