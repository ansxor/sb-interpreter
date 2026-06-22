---
name: sb-disasm
description: >
  Navigate the SmileBASIC 3.6.0 Ghidra disassembly to find an instruction's handler and read
  its actual ARM/VFP code — the authoritative source for the exact ALGORITHM (rounding mode,
  integer overflow/wrap, float→string format, RNG, error conditions). Use when writing or
  verifying a spec and you need behavior the docs don't pin down. Provides a Python navigator
  (name → xref → handler function → code).
metadata:
  tags: [smilebasic, disassembly, ghidra, arm, reverse-engineering, spec]
---

# sb-disasm — read the SmileBASIC 3.6.0 disassembly

The disassembly is the **authority on the algorithm**. The oracle (sb-oracle skill) gives
sampled outputs; the disassembly explains *why* and covers edge cases your samples miss
(exact rounding, overflow boundaries, float-format digits, the RNG/TinyMT path). Cite what
you read as a `disassembled` source in the spec.

## Hard facts
- Artifacts (in `sb-disassembly/`): `SmileBASIC_3.6.0_CIA.bin` (decompressed code),
  `listings/cia_3.6.0.lst` (disassembly text), `listings/cia_3.6.0.functions.txt` (6,558
  function bounds). These are gitignored but present locally; rebuild via the repo README.
- **runtime address = .bin file offset + 0x100000.** Segments: `.text [0x100000,0x2C8000)`,
  `.rodata [0x2C8000,0x2FD000)`, `.data [0x2FD000,0x325000)`.
- Command/function **names are UTF-16LE** in `.rodata`; a keyword table near `0x2C8E00`
  references them; handlers are functions in `.text`. (A plain `grep FLOOR` finds nothing —
  use the tool, which searches the UTF-16 bytes.)

## Tool — `tools/disasm.py`
```bash
cd .claude/skills/sb-disasm/tools
python3 disasm.py find FLOOR          # locate the name (UTF-16 + ASCII) -> addresses (+NUL-term)
python3 disasm.py handler FLOOR       # name -> xref -> candidate handler FUNCTIONS in .text
python3 disasm.py show 0x148858 60    # read N disassembly lines from a function (ARM/VFP)
python3 disasm.py func 0x148858       # which function contains an address (name + bounds)
python3 disasm.py xref 0x2ed8f4       # every 32-bit pointer TO an addr (tables + code pools)
python3 disasm.py near 0x<kwtbl> 16   # dump words around a table addr, classified TEXT/RODATA/DATA
```

## Workflow (per instruction)
1. `find NAME` → the name string address(es). Prefer the `<NUL-term>` `.rodata` hit (the real
   keyword entry); ignore substring hits (e.g. `SIN` inside `ASIN`).
2. `handler NAME` → candidate handler functions. Verify each with `show` — the right one
   reads/uses the args and does the relevant math (look for `vcvt`/`vsqrt`/`vmul` for floats,
   integer ops, calls to format/RNG helpers).
3. `show <handler>` → read the ARM/VFP and write down the exact behavior (rounding mode,
   overflow, format). Cite the handler address in the spec `sources:` as `type: disassembled`.

If `handler` finds nothing (the handler is index-dispatched off the sorted keyword table):
`xref <name-addr>` to find the table entry, then `near <entry>` to inspect the record and the
parallel function-pointer array; or `xref` a related error/format string the handler raises.

## When to use vs. skip
- USE for anything where the exact algorithm matters: rounding (FLOOR/ROUND/CEIL), integer
  overflow/wrap, `STR$`/PRINT float formatting, RND/RNDF/RANDOMIZE, bit ops, error conditions.
- For plain IEEE-double math (SIN/COS/EXP…) the handler will be standard VFP/libm — note that
  and lean on the oracle + docs; still cite the handler address you confirmed.
