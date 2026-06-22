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
python3 disasm.py dispatch FLOOR      # AUTHORITATIVE name -> handler (use this FIRST)
python3 disasm.py dispatch            # dump the whole builtin table (~217 name->handler rows)
python3 disasm.py show 0x1448b4 60    # read N disassembly lines from a function (ARM/VFP)
python3 disasm.py showmany cases.txt  # read MANY handlers in one call (lines: `ADDR [N] [label]`)
python3 disasm.py handler AND         # name -> handler; dispatch first, heuristic fallback
python3 disasm.py find FLOOR          # locate the name (UTF-16 + ASCII) -> addresses (+NUL-term)
python3 disasm.py func 0x148858       # which function contains an address (name + bounds)
python3 disasm.py xref 0x2ed8f4       # every 32-bit pointer TO an addr (tables + code pools)
python3 disasm.py near 0x<kwtbl> 16   # dump words around a table addr, classified TEXT/RODATA/DATA
```

## Workflow (per instruction)
1. `dispatch NAME` → the **authoritative** handler address, pinned from the builtin dispatch
   table (`(name_ptr, handler_ptr)` records in `.data`). One shot, no guessing — covers the
   ~217 dispatched builtins (functions + most commands).
2. `show <handler>` → read the ARM/VFP and write down the exact behavior (rounding mode,
   overflow, format; look for `vcvt`/`vsqrt`/`vmul` for floats, calls to format/RNG helpers).
   **This step is what `disassembled` certifies — the address alone is NOT.** Your
   `type: disassembled` ref MUST quote real body detail (an errnum site like `mov r0,#0x4`, a
   range guard `vcmpe …`, a constant, or ≥2 real addresses). A ref that is just an address +
   prose is rejected by `cargo test -p sb-spec` and got commit df691b1 reverted — if you only
   have the address, label it `confidence: hypothesis`, not `disassembled`.
3. Reading several handlers in a category? Put `ADDR N label` lines in a file and run
   `showmany` (or pipe via `showmany -`) — ONE call, no fragile bash `for`-loop quoting.

**Operators & special forms** (AND/OR/XOR/MOD/DIV, PRINT, PI, control keywords) are NOT in the
dispatch table — they're parsed specially. There `dispatch NAME` says so and `handler NAME`
falls back to the heuristic: `find NAME` → `xref <name-addr>` for the keyword-table entry →
`near <entry>` to inspect the record and parallel function-pointer array; or `xref` a related
error/format string the handler raises. If you still can't pin it, cite the name address and
mark that source `confidence: hypothesis`.

## When to use vs. skip
- USE for anything where the exact algorithm matters: rounding (FLOOR/ROUND/CEIL), integer
  overflow/wrap, `STR$`/PRINT float formatting, RND/RNDF/RANDOMIZE, bit ops, error conditions.
- For plain IEEE-double math (SIN/COS/EXP…) the handler will be standard VFP/libm — you STILL
  `show` the body (confirm the argcount/errnum checks and the VFP/libm call), quote that detail
  in the ref, and lean on the oracle + docs for the numeric result. "It's just libm" is not a
  reason to skip `show` and label it `disassembled` from the address alone.
