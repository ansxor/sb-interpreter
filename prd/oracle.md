# O — Oracle harvest engine (PRD)

> Status: 🔥 active — **connected** · Depends on: M0 · Read `prd/README.md`. Tasks: `PRD.md` (O-T*).

## Context / why

The oracle is real SmileBASIC 3.6.0 running in **Azahar** (the Citra fork), queried over
its RPC, used to capture **ground truth** (`hw_verified`). It's the engine behind the
spec build-out (S) and the `hw_verified` confidence tier. **It works today** — the
remaining tasks are about driving programs and capturing each output type.

Run the oracle **locally / supervised** (it's reachable from this machine). Never in CI —
PR CI replays the frozen fixtures harvest produces (see `harness/README.md`).

## Working setup (the runbook)

Azahar is at `/Applications/Azahar.app`; binary `/Applications/Azahar.app/Contents/MacOS/azahar`.

```bash
AZ=/Applications/Azahar.app/Contents/MacOS/azahar
# 1. install the 3.6.0 update CIA (one-time; done):
"$AZ" --install "0004000E0016DE00 SmileBASIC Ver.3.6.0 (CTR-U-JPKE) (U).cia"
# 2. RPC server: already enabled in
#    ~/Library/Application Support/Azahar/config/qt-config.ini  (enable_rpc_server=true)
# 3. boot the base app (Azahar applies the installed 3.6.0 update):
"$AZ" "000400000016DE00.00000004 SmileBASIC (CTR-N-JPKE) (U).cxi" &
# 4. connect (UDP 45987):
python3 -c 'import sys;sys.path.insert(0,"tools");from citra import Citra;c=Citra();print(c.process_list())'
```

Azahar CLI levers the harvest uses: `--install` · boot `<file>` · **`-r/--movie-record`
& `-p/--movie-play`** (TAS movies = deterministic input/autorun) · **`-d/--dump-video`**
(graphics capture) · `-g/--gdbport` and the RPC (memory read/write).

### Proven facts (O-T1)
- RPC connects; the SB process is `petitcom`, title `0x0004000000016DE00`.
- `read_memory(0x00100000, 16)` → `070000eb…` (real ARM).
- Banner at runtime `0x2E9AE0` = `"SMILEBASIC for Nintendo 3DS ver 3.6.0"` — confirms the
  build AND the mapping **runtime addr = disassembly file offset + 0x100000**. So any
  address found statically (ERRNUM, console grid, framebuffer, keyword/handler tables) is
  directly readable live.

Wrapper: `harness/oracle/citra_rpc.py` (`connect_smilebasic()`, typed reads).

## Tasks

### O-T1 — RPC connection ✅ done
Connected, read guest memory, confirmed 3.6.0 + the address mapping (above).

### O-T2 — Autorun
- **Approach:** drive SB to RUN a target program deterministically. Preferred: record a TAS
  movie (`-r`) of the key sequence (load slot → RUN), replay with `-p`. Alternative: RPC
  `write_memory` a trigger / direct-mode command. → O-T1
- **Acceptance:** one host command boots SB and runs a chosen program unattended, repeatably.

### O-T3 — Program injection
- **Approach:** get a test program into SB — either the extdata/project file format
  (`harness/corpus/sbsave/` already documents the PETC/project format via
  `tools/extract_sbsave.py`) written into Azahar's sdmc extdata, or RPC `write_memory` into
  a program slot. → O-T1
- **Acceptance:** a host-written program appears in SB and can be RUN.

### O-T4 — stdout capture
- **Approach:** read the console character grid from guest memory (RE its address in the
  disassembly; map via +0x100000), or scrape via `CHKCHR` in a harness program. → O-T1
- **Acceptance:** capturing the screen after a `PRINT` program yields the exact text.

### O-T5 — ERRNUM/ERRLINE capture
- **Approach:** RE the ERRNUM/ERRLINE sysvar addresses; read after a halt. Set them in
  `harness/oracle/citra_rpc.py`. Settles documented-vs-real errnum questions. → O-T1
- **Acceptance:** error programs report the correct errnum/errline read from memory.

### O-T6 — Framebuffer capture
- **Approach:** `--dump-video` for whole-frame capture, and/or RE the top/bottom framebuffer
  addresses + pixel format (likely tiled) and read via RPC; decode to RGBA8888. → O-T1
- **Acceptance:** capturing a known graphics screen yields RGBA matching the display; feeds M2.

### O-T7 — Audio capture
- **Approach:** capture emulator audio (`--dump-video` carries audio, or loopback) as PCM. → O-T1
- **Acceptance:** capturing a known BGM yields PCM; feeds M5.

### O-T8 — harvest.py end-to-end
- **Approach:** wire O-T2..O-T7 into `harness/harvest/harvest.py`: run each spec/corpus case,
  capture, write `spec/tests/<id>.yaml` (`hw_verified`) + golden PNG/WAV, drain
  `HARVEST_QUEUE.md`. Commit fixtures; open a PR. → O-T2, O-T3, O-T4, O-T5
- **Acceptance:** a harvest run produces committed fixtures the deterministic gate replays;
  `harness/oracle/*` has no remaining `NotImplementedError`.

## Risks / open questions
- Emulator fidelity ≠ hardware in rare cases; cross-check suspicious results vs real-HW logs.
- First-boot/region state of SB under emulation may need a one-time TAS to reach the editor.
