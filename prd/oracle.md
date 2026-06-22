# O — Oracle engine (PRD)

> Status: 🔥 **value harvest works** (the `sb-oracle` skill); errnum + framebuffer + audio
> capture TODO · Depends on: M0 · Read `prd/README.md`. Tasks: `PRD.md` (O-T*).

## Context / why

The oracle is real SmileBASIC 3.6.0 running in **Azahar**, used to capture **ground truth**
(`hw_verified`) for spec build-out (S). It is implemented as the **`.claude/skills/sb-oracle/`
skill** — full mechanism + the cracked file format live in `SKILL.md` there (and in the
`sb-file-format-oracle` memory).

What we ruled out the hard way: Azahar has **no InputRedirection**, and **heavy RPC reads
crash/reset SB**. So the working mechanism is **cliclick synthetic touch** on the focused
window + **extdata files on disk** for program input and result output (no OCR).

Run it locally/supervised; never in CI — PR CI replays the frozen fixtures harvest produces.

## How to drive it
```bash
cd .claude/skills/sb-oracle/tools
python3 run_case.py prog 'FLOOR(-2.1)'             # -> -3   (write program -> LOAD,0 -> RUN -> read file)
python3 run_case.py expr 'MID$("ABCDE",2,3)' str   # typed-command path
```
Requires Azahar running with SB on the DIRECT-mode screen. The file format (header markers +
HMAC-SHA1 footer key, recovered from `nnn1590/lpp-3ds-sbfm`) is in `sb_extdata.py`; verified
against real SB-saved files and by load+run of our written programs.

## Tasks

- [x] **O-T1 RPC connection** — confirmed 3.6.0; runtime = disassembly file offset + 0x100000.
  (RPC is now used only for small reads; the skill drives I/O via cliclick + extdata.)
- [x] **O-T2 Autorun** — `LOAD"PRG0:P",0` (auto-dismiss) + `RUN`, typed via cliclick.
- [x] **O-T3 Program injection** — write a valid extdata file (header + HMAC-SHA1 footer).
- [x] **O-T4 Value/stdout capture** — program `SAVE"TXT:O",STR$(...)`; read `body[80:-20]` off disk.
- [ ] **O-T5 ERRNUM/ERRLINE capture** — error cases halt with NO result file (current `run_case`
  just times out). Detect the halt and read the error: RE the ERRNUM sysvar address (near
  MAINCNT @0x315EC0) for a small RPC read, or screenshot+parse the error dialog. Needed for
  error-expecting spec tests and S-T14.
- [ ] **O-T6 Framebuffer capture** — `azahar --dump-video` and/or RE the top/bottom framebuffer
  addresses + pixel format; decode to RGBA8888 (graphics goldens for M2/M3).
- [ ] **O-T7 Audio capture** — emulator audio dump → PCM (audio goldens for M5).
- [ ] **O-T8 harvest end-to-end** — wire `run_case` into `harness/harvest/harvest.py`: batch a
  category's spec/corpus cases, capture, write `spec/tests/<id>.yaml` (`hw_verified`) + golden
  media, open a PR. The deterministic gate then replays them without the emulator. → O-T5

## Risks / open questions
- Emulator fidelity ≠ hardware in rare cases; cross-check suspicious results vs real-HW logs.
- cliclick needs Azahar frontmost+unoccluded (`open -a Azahar` first); keep the window at the
  pinned geometry so key coordinates stay valid.
