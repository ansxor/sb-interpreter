# sb-interpreter

A **faithful, bit-accurate reimplementation of SmileBASIC 3.6.0** (the Nintendo 3DS
app) in Rust — targeting both native desktop and **WebAssembly** (the SmileBASIC
community is web-based). "Faithful" means matching not just the language but the
*limitations*: the RGBA5551 layered screen, 4 program slots, TinyMT RNG, exact error
numbers, MML audio, sprite/BG hardware behavior. Where real SB3 and this differ, that's
a bug here.

Open behaviors still awaiting emulator-oracle verification are tracked in
[`HARVEST_QUEUE.md`](HARVEST_QUEUE.md).

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

## Status

Milestones **M0–M7 complete**: spec pipeline, the language core (lexer → parser →
compiler → bytecode VM, TinyMT RNG, console) running native + wasm, graphics (RGBA5551
compositor), sprites/BG, input/timing, MML audio, files/projects/system, and hardening
(fuzzing, exact error strings, float formatting, overflow/precision). Remaining work is
oracle-pending fidelity refinements tracked in `HARVEST_QUEUE.md`.
