# tools/

Host-side utilities for building and driving the reference assets. These are glue
scripts (Python), distinct from the interpreter itself (`crates/`) and the conformance
harness (`harness/`).

| Tool | What it does |
|---|---|
| `citra.py` | Client for the Citra/Azahar **scripting RPC** (UDP `127.0.0.1:45987`): `read_memory` / `write_memory` / `process_list` / `set_process`. The foundation the conformance oracle is built on (`harness/oracle/`). Run `python3 citra.py` to self-test via doctests against a running emulator. |
| `extract_code.py` | Parses the 3DS CXI/CIA containers in the repo root and extracts the ARM `.code` segment from each NCCH/ExeFS. Produces the `*_code.bin` files. |
| `extract_sbsave.py` | Decodes the `sbsave/` PETC server scrape (915 downloads → 3,329 programs + 2,773 resources) into an extdata-compatible tree under `harness/corpus/sbsave/` + a committed `INDEX.json` manifest. The docstring documents the reverse-engineered PETC/project file format. `--get KEY[/NAME]` prints one program. |

## Related (not moved — live next to their data)

- `../sb-disassembly/decompress_blz.py` — 3DS backward-LZSS (BLZ) decompressor with the
  validated `disp_add=3`. Turns the raw extracted `.code` into the decompressed image
  Ghidra analyzed. See `../sb-disassembly/README.md` for the full pipeline.

## Emulator notes

- Works against either **Citra** or its active fork **Azahar** — the scripting RPC
  protocol is identical. The RPC server must be enabled in the emulator's settings.
- Sanity check a live connection:
  ```python
  from citra import Citra
  c = Citra()
  print(c.process_list())          # find the SmileBASIC process
  print(c.read_memory(0x100000, 4))  # first ARM word of .code
  ```
