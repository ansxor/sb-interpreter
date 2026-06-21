# sb-interpreter

A **faithful, bit-accurate reimplementation of SmileBASIC 3.6.0** (the Nintendo 3DS
app) in Rust — targeting both native desktop and **WebAssembly** (the SmileBASIC
community is web-based). "Faithful" means matching not just the language but the
*limitations*: the RGBA5551 layered screen, 4 program slots, TinyMT RNG, exact error
numbers, MML audio, sprite/BG hardware behavior. Where real SB3 and this differ, that's
a bug here.

Full plan: `~/.claude/plans/i-want-to-make-bright-plum.md`. Task breakdown: [`PRD.md`](PRD.md)
(canonical task list) with per-milestone design docs in [`prd/`](prd/README.md).

## How fidelity is enforced

Three reference sources, ranked by a **confidence ladder**
(`documented < community < observed < disassembled < hw_verified`):

- **`sb-docs/`** — official docs (248 instructions + reference tables) → the
  *documented* layer. Ingested into `spec/` by `tools/gen_specs.py`.
- **`sb-disassembly/`** — Ghidra disassembly of the real 3.6.0 binary → reverse-engineer
  exact numeric behavior (*disassembled* layer).
- **emulator oracle** (`harness/`) — real SB3 in Citra/Azahar → ground truth
  (*hw_verified* layer). `osb/` (otya128's D interpreter, 3.5.0) is a behavioral
  cross-check only.

The `spec/` tree is the single source of truth; its tests drive a **deterministic**
conformance suite. Expensive oracle/fuzzer work runs **offline** to harvest committed
golden fixtures; **CI replays only those fixtures** (no emulator, no fuzzing — see
`harness/README.md`).

## Layout

```
crates/
  sb-core            language core: lexer/parser/compiler/bytecode VM, Value, builtins (wasm-safe)
  sb-render          RGBA5551 layer compositor -> framebuffer (display + oracle pixel-diff)
  sb-audio           MML parser + synth (M5)
  sb-platform-native winit desktop runner (bin: `sb`)
  sb-platform-wasm   canvas/WebGL browser runner
  sb-spec            spec loader + coverage reporter (bin: `sb-spec-coverage`)
spec/                YAML specs (source of truth) + reference tables + test overlays
harness/             conformance oracle, differential testing, fuzzer, corpus
tools/               citra.py (emulator RPC), extract_code.py, gen_specs.py
sb-docs/             offline docs mirror      (reference)
sb-disassembly/      Ghidra workspace         (reference)
```

`osb/` and the 3DS ROM images / PDFs are git-ignored (copyrighted or large/derived; see
`.gitignore`). `osb/` is an external checkout of <https://github.com/otya128/osb>.

## Quickstart

```bash
cargo test --workspace                              # deterministic gate (units + 248-spec conformance)
cargo run -p sb-spec --bin sb-spec-coverage         # fidelity dashboard
cargo run -p sb-platform-native --bin sb            # native runner (smoke test for now)
cargo build --workspace --target wasm32-unknown-unknown
python3 tools/gen_specs.py                           # regenerate spec/ from sb-docs/
python3 harness/diff/replay.py                       # deterministic replay inventory
```

## Autonomous development (Ralph loop)

`./ralph.sh` runs a [Ralph](https://ghuntley.com/ralph/) loop: each iteration spawns a
fresh `claude -p` agent that picks the next doable task from `PRD.md`, implements only
that, runs the verification gate, checks it off, and commits. `./ralph.sh 5` caps
iterations; `touch ralph.stop` stops cleanly; logs land in `ralph-logs/`. It runs
unattended with `--dangerously-skip-permissions` and commits every productive iteration.

## Status

Milestone **M0 complete**: workspace + spec pipeline (248 instructions, reference
tables) + two-phase harness skeleton + CI. Next: **M1** — the language core (lexer →
parser → compiler → VM, TinyMT RNG, console) running in a window, validated against the
oracle. See the plan for M2–M7 (graphics, sprites/BG, input/timing, audio, files,
hardening).
