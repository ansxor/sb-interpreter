#!/usr/bin/env python3
"""Locate a SmileBASIC instruction in the 3.6.0 disassembly + dump listing regions.

Makes consulting the disassembly a one-liner so specs can carry a `disassembled` source.

Usage:
  python3 tools/disasm.py find NAME     # find the instruction NAME (rendered + raw UTF-16) -> addresses
  python3 tools/disasm.py show ADDR [N] # print N listing lines starting at a runtime address (hex)
  python3 tools/disasm.py grep PATTERN  # raw grep the listing

Notes: names are UTF-16; the listing address IS the runtime address (= disasm file offset +
0x100000). The listing is ~34 MB. Ghidra labels long names as `unicode u"NAME"`; short ones
are found via their raw UTF-16LE hex bytes (the listing's hexbytes column).
"""
import subprocess
import sys
from pathlib import Path

LST = Path(__file__).resolve().parent.parent / "sb-disassembly" / "listings" / "cia_3.6.0.lst"


def _grep(args):
    if not LST.exists():
        sys.exit(f"listing not found: {LST} (build it — see sb-disassembly/README.md)")
    return subprocess.run(["grep", *args, str(LST)], capture_output=True, text=True).stdout


def find(name):
    u = name.upper()
    print(f"# instruction name {u!r} in {LST.name}")
    rendered = _grep(["-an", f'unicode u"{u}"']).splitlines()
    hexbytes = "".join(f"{ord(c):02x}00" for c in u)  # UTF-16LE, lowercase like the listing
    raw = _grep(["-ain", hexbytes]).splitlines()
    seen = set()
    for line in rendered + raw:
        if line in seen:
            continue
        seen.add(line)
        print(" ", line.strip()[:140])
    if not seen:
        print("  (not found as a wide string — try `grep` with other spellings, or the keyword")
        print("   table near 0x2C8E00 / name pool ~0x2ED800; the handler is a separate FUN_*)")
    print(f"\n# next: `disasm.py show <addr>` to read the region; the keyword table (~0x2C8E00)")
    print(f"#       maps names to handlers. Cite the address you used as a `disassembled` source.")


def show(addr, n=40):
    a = addr.lower().replace("0x", "").lstrip("0").zfill(8).upper()
    # listing lines look like "650502:002ED8F4  <hexbytes>  <disasm>"
    out = _grep(["-an", "-A", str(n), a]).splitlines()
    if not out:
        out = _grep(["-an", "-A", str(n), a.lower()]).splitlines()
    print("\n".join(out[: n + 5]) if out else f"(address {a} not found in listing)")


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__)
        return
    if a[0] == "find":
        find(a[1])
    elif a[0] == "show":
        show(a[1], int(a[2]) if len(a) > 2 else 40)
    elif a[0] == "grep":
        print(_grep(["-an", a[1]])[:8000])
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
