# O — Emulator-oracle bring-up (PRD)

> Status: ⬜ · Depends on: M0 (otherwise independent — can start now) · Read `prd/README.md`. Tasks: `PRD.md` (O-T*).

## Context / why

The oracle is real SmileBASIC 3.6.0 running in Citra/Azahar, queried over the scripting
RPC, used to capture **ground truth** (`hw_verified`). It's the engine behind Phase-A
harvest. This track is independent of the interpreter — an agent with the emulator can
resolve all of it in parallel with M1+. Each spike below unblocks a specific downstream
need; see the plan's "Spikes" section.

## Goal (Definition of Done)

`harness/harvest/harvest.py` can take a test program, run it on real SB3, and capture its
stdout/values/errors (and, for M2/M5, framebuffer/audio), then write committed fixtures
(`spec/tests/` overlays + `harness/corpus/golden/`). All capture paths in
`harness/oracle/` are implemented (no more `NotImplementedError`).

## Reference sources
- `tools/citra.py` (RPC) + `harness/oracle/citra_rpc.py` (wrapper).
- Disassembly for addresses (ERRNUM/ERRLINE, console grid, framebuffers) — remember
  runtime addr = file offset + `0x100000`; command names are UTF-16. Open
  `sb-disassembly/ghidra_project/sb-3ds.gpr` in the GUI for xref tracing.
- Emulator docs for enabling the scripting RPC + locating extdata on disk.

## Tasks

### O-T1 — Confirm RPC connection
- **Files:** `harness/oracle/citra_rpc.py`.
- **Approach:** with SB3 running, verify `connect_smilebasic()` finds the process and
  `read_memory(0x100000, 4)` returns the expected first ARM word. Document emulator setup
  (which build, how to enable RPC, port).
- **Acceptance:** a script prints the SB process + first code word; setup documented in
  `tools/README.md`.

### O-T2 — Autorun
- **Approach:** find how to auto-start a program in SB3 under emulation — TXT autoload, a
  scripted button-input macro to navigate to RUN, or an RPC-triggered RUN (write a flag /
  jump). Pick the most reliable. → O-T1
- **Acceptance:** a host script causes a known program to run start-to-finish unattended.

### O-T3 — extdata container format
- **Approach:** determine SB3's on-disk format for stored programs/files in extdata (locate
  the emulator's extdata dir; RE the container from the disassembly or community notes) so
  the host can inject test programs by writing files. Fallback: RPC `write_memory` into a
  program slot.
- **Acceptance:** a host-written program appears in SB's FILES and can be loaded/run.

### O-T4 — stdout capture
- **Approach:** capture console output — either scrape the grid in-SB via `CHKCHR(x,y)`
  from a harness-loader program, or read the console grid region from emulator memory via
  RPC (RE the address). → O-T1/O-T2
- **Acceptance:** capturing the screen after a `PRINT` program yields the exact text.

### O-T5 — ERRNUM/ERRLINE capture
- **Files:** set `ERRNUM_ADDR`/`ERRLINE_ADDR` in `citra_rpc.py`.
- **Approach:** RE the addresses of the ERRNUM/ERRLINE system variables; after running an
  error-triggering program, read them. This settles documented-vs-real errnum questions
  (e.g. FLOOR("x")). → O-T1
- **Acceptance:** error programs report the correct errnum/errline read from memory.

### O-T6 — Framebuffer capture
- **Files:** `harness/oracle/framebuffer.py`.
- **Approach:** RE the top/bottom framebuffer base addresses + on-device pixel format
  (likely tiled); implement read + detile + convert to RGBA8888. → O-T1
- **Acceptance:** capturing a known graphics screen yields an RGBA image matching what's
  displayed; feeds M2-T5 golden PNGs.

### O-T7 — Audio capture
- **Files:** `harness/oracle/audio.py`.
- **Approach:** capture emulator audio output (audio dump / loopback) as PCM for diffing.
  → O-T1
- **Acceptance:** capturing a known BGM yields PCM; feeds M5-T6 golden audio.

### O-T8 — harvest end-to-end
- **Files:** `harness/harvest/harvest.py`, `harness/diff/run.py`.
- **Approach:** wire O-T2..O-T7 into harvest: run each spec/corpus case, capture results,
  write `spec/tests/` overlays (expects) + golden PNG/WAV, bump confidence to
  `hw_verified`. Open a PR with refreshed fixtures (never in PR CI). → O-T2, O-T4, O-T5
- **Acceptance:** a harvest run produces committed fixtures that the deterministic Phase-B
  gate replays; `harness/oracle/*` has no remaining `NotImplementedError`.

## Milestone verification
```bash
# with emulator + SB3 running:
python3 harness/oracle/citra_rpc.py        # (or a small connect test)
python3 harness/harvest/harvest.py         # produces spec/tests + golden fixtures
# then, hermetic:
python3 harness/diff/replay.py             # replays what harvest produced
```

## Risks / open questions
- Emulator fidelity: the oracle is only as faithful as Citra/Azahar. Cross-check
  suspicious results against real hardware where possible.
- Headless/cron runs may lack the interactively-authenticated emulator session — harvest
  is intended to be maintainer-run locally.
