# harness/ — conformance oracle + differential testing

Two phases, deliberately separated (see the project plan and the
`deterministic-golden-testing` rule):

```
            ┌─────────────────────── PHASE A: HARVEST (offline, slow, non-deterministic)
            │  real SmileBASIC 3.6.0 in Citra/Azahar  +  the fuzzer
            │      │
            │      ├── oracle/   drive SB, capture text/values/errors/framebuffer/audio
            │      ├── fuzz/     generate programs, diff vs oracle, minimize failures
            │      └── harvest/  write results back as COMMITTED fixtures:
            │                      • spec/tests/<id>.yaml  expect: blocks  (-> hw_verified)
            │                      • harness/corpus/golden/gfx/*.png
            │                      • harness/corpus/golden/audio/*.wav
            │                      • promoted fuzz cases -> harness/corpus/
            ▼
   committed fixtures (in git)
            ▲
            │  ┌──────────────────── PHASE B: REPLAY (deterministic, hermetic, every commit)
            └──┤  diff/replay.py + `cargo test`  run sb-core against the committed
               │  fixtures only — NO emulator, NO fuzzer, NO network, fixed RNG seeds.
               └────────────────────  This is the PR gate.
```

**Never run the emulator or the fuzzer in PR CI.** Contributors (and LLMs) validate a
change with `cargo test` against frozen goldens. Phase A is run by a maintainer or on a
schedule, and opens a PR with refreshed fixtures.

## Directories

| Dir | Phase | What |
|---|---|---|
| `oracle/`  | A | Drive real SmileBASIC via the Citra/Azahar RPC (`tools/citra.py`): `extdata` text/value/error capture, `framebuffer` pixel reads, `audio` dump. |
| `fuzz/`    | A | Grammar-aware, **seeded** program generator for differential testing. |
| `harvest/` | A | Orchestrates oracle+fuzz and writes committed fixtures. Opens the refresh PR. |
| `diff/`    | B+A | `replay.py` (deterministic gate) and `run.py` (3-way differential: sb-core vs osb vs oracle). |
| `corpus/`  | — | Committed test programs, cross-cutting cases, and golden PNG/WAV fixtures. |

## Requirements

- Phase B (replay): just the Rust toolchain. Hermetic.
- Phase A (harvest): Citra or Azahar with the **scripting RPC enabled** (UDP 45987),
  SmileBASIC 3.6.0 installed, and (optionally) `pyyaml`. See `tools/README.md`.
