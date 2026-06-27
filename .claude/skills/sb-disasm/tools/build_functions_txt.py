#!/usr/bin/env python3
"""Rebuild `cia_3.6.0.functions.txt` (the function-bounds table disasm.py needs for
`func`/`xref`/`near`/`dispatch` labels) directly from the Ghidra listing export
`cia_3.6.0.lst`.

The listing already carries one `; ======== FUNCTION: <name>  <sig> ========` header per
function, immediately followed by the function's first address line. So the bounds table is
fully derivable from the listing — no live Ghidra / analyzeHeadless install required (the
.lst was itself exported from the same ghidra_project/sb-3ds).

Output line format (what disasm.py `_funcs()` parses):  `<hexaddr> <decimal_size> <name>`
Size = next function's start - this function's start (contiguous-in-listing bound, matching
how `_func_at` does a bisect + `a <= addr < a+sz` range test, with a `(+gap)` fallback).

Usage:  python3 build_functions_txt.py            # writes the table next to the .lst
        python3 build_functions_txt.py --stdout   # print, don't write
"""
import re
import sys
from pathlib import Path

DIS = Path(__file__).resolve().parents[4] / "sb-disassembly"
LST = DIS / "listings" / "cia_3.6.0.lst"
OUT = DIS / "listings" / "cia_3.6.0.functions.txt"
TEXT_END = 0x002C8000  # CIA 3.6.0 .text upper bound; caps the final function's size

HDR = re.compile(r"^; =+ FUNCTION: (\S+)\s")
ADDR = re.compile(r"^([0-9A-Fa-f]{8})\b")


def parse(lst_text):
    """Yield (start_addr, name) for each function, in listing order."""
    pending = None
    for line in lst_text.splitlines():
        m = HDR.match(line)
        if m:
            pending = m.group(1)
            continue
        if pending is not None:
            a = ADDR.match(line)
            if a:
                yield (int(a.group(1), 16), pending)
                pending = None


def main():
    if not LST.exists():
        sys.exit(f"missing {LST}")
    funcs = list(parse(LST.read_text(errors="replace")))
    funcs.sort()
    lines = ["; addr            size    name  (rebuilt from cia_3.6.0.lst FUNCTION headers)"]
    for i, (addr, name) in enumerate(funcs):
        end = funcs[i + 1][0] if i + 1 < len(funcs) else TEXT_END
        size = max(end - addr, 0)
        lines.append(f"{addr:08x} {size} {name}")
    text = "\n".join(lines) + "\n"
    if "--stdout" in sys.argv:
        sys.stdout.write(text)
    else:
        OUT.write_text(text)
        print(f"wrote {OUT} ({len(funcs)} functions)")


if __name__ == "__main__":
    main()
