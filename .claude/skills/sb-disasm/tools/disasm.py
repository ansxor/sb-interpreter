#!/usr/bin/env python3
"""Navigate the SmileBASIC 3.6.0 disassembly: name -> xref -> handler function -> code.

The decompressed code (`SmileBASIC_3.6.0_CIA.bin`) is loaded at runtime base 0x00100000, so
  runtime_addr = file_offset + 0x100000.
Segments (CIA 3.6.0): .text [0x100000,0x2C8000) · .rodata [0x2C8000,0x2FD000) · .data
[0x2FD000,0x325000). Instruction/command names are stored as NUL-terminated UTF-16LE in
.rodata; a keyword table near 0x2C8E00 references them; handlers are functions in .text.

Commands:
  find  NAME            locate a command/string name in .bin (UTF-16LE + ASCII) -> addresses
  xref  ADDR            every 32-bit little-endian pointer TO addr (refs: tables + code pools)
  near  ADDR [N]        dump N words around addr, classified (TEXT/RODATA/DATA/int)
  func  ADDR            the function containing addr (name + bounds), from functions.txt
  show  ADDR [N]        N disassembly lines from the .lst starting at addr (read the code)
  handler NAME          find NAME -> xref -> surface candidate handler functions in .text

Typical flow to find an instruction's behavior:
  disasm.py find FLOOR            # -> name @ 0x2ED8F4
  disasm.py handler FLOOR         # -> candidate handler function(s)
  disasm.py show 0x<handler> 60   # read the ARM/VFP math
Cite the address you used as a `disassembled` source in the spec.
"""
import bisect
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]  # tools/sb-disasm/skills/.claude/<repo>
DIS = ROOT / "sb-disassembly"
BIN = DIS / "SmileBASIC_3.6.0_CIA.bin"
LST = DIS / "listings" / "cia_3.6.0.lst"
FUNCS = DIS / "listings" / "cia_3.6.0.functions.txt"
BASE = 0x00100000
TEXT = (0x100000, 0x2C8000)
RODATA = (0x2C8000, 0x2FD000)


def _data():
    if not BIN.exists():
        sys.exit(f"missing {BIN} (build it — see sb-disassembly/README.md)")
    return BIN.read_bytes()


def _funcs():
    out = []
    for line in FUNCS.read_text().splitlines():
        if line.startswith(";") or not line.strip():
            continue
        a, sz, name = line.split(None, 2)
        out.append((int(a, 16), int(sz), name))
    out.sort()
    return out


def _seg(a):
    if TEXT[0] <= a < TEXT[1]:
        return "TEXT"
    if RODATA[0] <= a < RODATA[1]:
        return "RODATA"
    if RODATA[1] <= a < BASE + 0x225000:
        return "DATA"
    return "?"


def _func_at(addr, funcs):
    i = bisect.bisect_right([f[0] for f in funcs], addr) - 1
    if 0 <= i < len(funcs):
        a, sz, name = funcs[i]
        if a <= addr < a + sz:
            return (a, sz, name)
        if a <= addr:  # in a gap after a function (no size info)
            return (a, sz, name + " (+gap)")
    return None


def _find_bytes(data, pat):
    out, i = [], data.find(pat)
    while i != -1:
        out.append(i + BASE)
        i = data.find(pat, i + 1)
    return out


def cmd_find(name):
    data = _data()
    u = name.upper()
    u16 = u.encode("utf-16-le")
    print(f"# {u!r}  UTF-16LE hits:")
    for a in _find_bytes(data, u16):
        # require NUL-terminated (real string, not a substring)
        off = a - BASE + len(u16)
        term = data[off:off + 2] == b"\x00\x00"
        print(f"  {a:#08x} ({_seg(a)}){'  <NUL-term>' if term else ''}")
    asc = _find_bytes(data, u.encode())
    if asc:
        print(f"# ASCII hits: " + " ".join(f"{a:#08x}" for a in asc[:8]))


def cmd_xref(addr):
    data = _data()
    a = int(addr, 16) if isinstance(addr, str) else addr
    funcs = _funcs()
    print(f"# pointers to {a:#08x}:")
    for loc in _find_bytes(data, struct.pack("<I", a)):
        f = _func_at(loc, funcs) if _seg(loc) == "TEXT" else None
        tag = f"  in {f[2]} @{f[0]:#x}" if f else ""
        print(f"  {loc:#08x} ({_seg(loc)}){tag}")


def cmd_near(addr, n=16):
    data = _data()
    a = int(addr, 16) if isinstance(addr, str) else addr
    funcs = _funcs()
    start = a - 8
    for k in range(n):
        p = start + k * 4
        off = p - BASE
        if off < 0 or off + 4 > len(data):
            continue
        v = struct.unpack_from("<I", data, off)[0]
        seg = _seg(v)
        note = ""
        if seg == "TEXT":
            f = _func_at(v, funcs)
            note = f"-> {f[2]} @{f[0]:#x}" if f else "-> TEXT"
        elif seg in ("RODATA", "DATA"):
            note = f"-> {seg}"
        elif v < 0x10000:
            note = f"(int {v})"
        mark = " <==" if p == a else ""
        print(f"  {p:#08x}: {v:#010x} {note}{mark}")


def cmd_func(addr):
    a = int(addr, 16) if isinstance(addr, str) else addr
    f = _func_at(a, _funcs())
    print(f"{a:#08x} -> {f[2]}  [{f[0]:#x}..{f[0]+f[1]:#x}] ({f[1]} bytes)" if f else f"{a:#08x}: no function")


def cmd_show(addr, n=40):
    a = int(addr, 16) if isinstance(addr, str) else addr
    target = f"{a:08X}"
    out, hit = [], False
    for line in LST.read_text(errors="replace").splitlines():
        if not hit and target in line:
            hit = True
        if hit:
            out.append(line)
            if len(out) >= n:
                break
    print("\n".join(out) if out else f"(addr {target} not found in listing)")


def cmd_handler(name):
    data = _data()
    funcs = _funcs()
    u = name.upper()
    names = [a for a in _find_bytes(data, u.encode("utf-16-le"))
             if data[a - BASE + len(u) * 2: a - BASE + len(u) * 2 + 2] == b"\x00\x00"]
    if not names:
        print(f"# {u!r}: name string not found as a wide string. Try `find`/`grep`.")
        return
    print(f"# {u!r} name @ {', '.join(hex(a) for a in names)}")
    cand = {}
    for na in names:
        for loc in _find_bytes(data, struct.pack("<I", na)):
            # the keyword-table entry; scan a window for adjacent TEXT pointers (handlers)
            for p in range(loc - 0x10, loc + 0x14, 4):
                off = p - BASE
                if 0 <= off + 4 <= len(data):
                    v = struct.unpack_from("<I", data, off)[0]
                    if TEXT[0] <= v < TEXT[1]:
                        f = _func_at(v, funcs)
                        if f:
                            cand[f[0]] = f[2]
            # code that references the name directly (literal pool in a handler)
            if _seg(loc) == "TEXT":
                f = _func_at(loc, funcs)
                if f:
                    cand[f[0]] = f[2] + " (refs name in code)"
    if cand:
        print("# candidate handler functions (verify with `show`):")
        for a in sorted(cand):
            print(f"  {a:#08x}  {cand[a]}")
    else:
        print("# no adjacent TEXT pointer found near the keyword entry — handler is likely")
        print("# index-dispatched. Inspect the table with `near <name-xref>` and the parallel")
        print("# function-pointer array, or search code that references a related error string.")


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__)
        return
    c = a[0]
    arg = a[1] if len(a) > 1 else None
    n = int(a[2]) if len(a) > 2 else None
    if c == "find":
        cmd_find(arg)
    elif c == "xref":
        cmd_xref(arg)
    elif c == "near":
        cmd_near(arg, n or 16)
    elif c == "func":
        cmd_func(arg)
    elif c == "show":
        cmd_show(arg, n or 40)
    elif c == "handler":
        cmd_handler(arg)
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
