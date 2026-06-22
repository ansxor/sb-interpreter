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
  showmany FILE         read many handlers in ONE call; each line: `ADDR [N] [label]`
                        (or `-` to read the list from stdin). Avoids fragile bash for-loops.
  dispatch [NAME]       AUTHORITATIVE name -> handler from the builtin dispatch table.
                        No NAME = dump the whole table (~217 builtins). USE THIS FIRST.
  handler NAME          name -> handler (consults `dispatch`, then falls back to a heuristic
                        for operators/special forms the table doesn't cover, e.g. AND/PRINT).

Typical flow to find an instruction's behavior:
  disasm.py dispatch FLOOR        # -> handler=0x1448b4 (authoritative, one shot)
  disasm.py show 0x1448b4 60      # read the ARM/VFP math
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
DATA = (0x2FD000, 0x325000)


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


def _first_arm_line(addr, scan=14):
    """A representative ARM line from the handler body, condensed for a citation example
    (e.g. `cmp r0,#0x1` / `vcmpe.f64 d0,d1`). Returns a placeholder if the listing is
    missing. Used to seed the paste-ready `disassembled` ref so it carries real evidence."""
    a = int(addr, 16) if isinstance(addr, str) else addr
    if not LST.exists():
        return "mov r0,#0x4 (errnum) — read the body"
    target = f"{a:08X}"
    hit = False
    for line in LST.read_text(errors="replace").splitlines():
        if not hit and target in line:
            hit = True
        if hit:
            # listing rows look like `00149258 e3500001  cmp r0,#0x1`; grab the mnemonic+ops
            parts = line.split(None, 2)
            if len(parts) == 3 and all(c in "0123456789abcdefABCDEF" for c in parts[1]):
                return parts[2].strip()
    return "mov r0,#0x4 (errnum) — read the body"


def _wstr_at(data, addr, maxlen=24):
    """Decode a NUL-terminated UTF-16LE string at a runtime addr; None if it isn't a
    plausible (printable-ASCII) command name. Used to read dispatch-table name pointers."""
    o = addr - BASE
    out = []
    while 0 <= o < len(data) - 1:
        ch = struct.unpack_from("<H", data, o)[0]
        if ch == 0:
            break
        if ch < 0x20 or ch > 0x7E:   # command names are printable ASCII in UTF-16
            return None
        out.append(chr(ch))
        o += 2
        if len(out) > maxlen:
            return None
    return "".join(out) if out else None


_DISPATCH = None


def _dispatch_table(data):
    """The builtin dispatch table: a flat array of (name_ptr→RODATA, handler_ptr→TEXT)
    8-byte records in .data. Returns {NAME: [handler_addr, ...]}. Authoritative for the
    ~217 dispatched builtins (functions + most commands); does NOT cover operators and
    special-form keywords (AND/OR/MOD/PRINT/PI…), which are parsed/handled specially."""
    global _DISPATCH
    if _DISPATCH is not None:
        return _DISPATCH
    tbl = {}
    a = DATA[0]
    while a < DATA[1] - 8:
        p1 = struct.unpack_from("<I", data, a - BASE)[0]
        p2 = struct.unpack_from("<I", data, a - BASE + 4)[0]
        if RODATA[0] <= p1 < RODATA[1] and TEXT[0] <= p2 < TEXT[1]:
            s = _wstr_at(data, p1)
            if s and s[0].isalpha():
                tbl.setdefault(s, [])
                if p2 not in tbl[s]:
                    tbl[s].append(p2)
        a += 4
    _DISPATCH = tbl
    return tbl


def cmd_dispatch(name=None):
    data = _data()
    tbl = _dispatch_table(data)
    if name:
        u = name.upper()
        hits = tbl.get(u)
        if hits:
            funcs = _funcs()
            for h in hits:
                f = _func_at(h, funcs)
                fn = f[2] if f else "?"
                print(f"{u}\thandler={h:#08x}\t{fn}")
            # The address ALONE is not a citation. Auto-show the body so the next thing in
            # your context is real ARM, and emit a paste-ready `disassembled` ref skeleton
            # built from an actual listing line — the spec gate (sb-spec specs_load) rejects
            # a `disassembled` source that carries no body evidence (mnemonic / ≥2 addrs).
            h0 = hits[0]
            print(f"\n# --- {u} handler body @{h0:#08x} (first 14 lines) ---")
            print("# `disassembled` means you READ THIS. Cite a real line below, not docs prose.")
            cmd_show(h0, 14)
            arm = _first_arm_line(h0)
            print(f"\n# paste-ready source (fill in the behavior FROM the body above):")
            print(f'  - {{ type: disassembled, ref: "cia_3.6.0.lst {u} handler @{h0:#08x}; '
                  f'<errnum/range/rounding from the body, e.g. {arm}>" }}')
        else:
            print(f"# {u!r} not in the dispatch table — it's likely an operator or special-")
            print(f"# form keyword (AND/OR/MOD/PRINT/PI…) handled in the parser, not dispatched.")
            print(f"# Fall back to `handler {u}` (heuristic) / `find`+`xref`.")
        return
    print(f"# builtin dispatch table: {len(tbl)} names (name -> handler)")
    funcs = _funcs()
    for n in sorted(tbl):
        for h in tbl[n]:
            f = _func_at(h, funcs)
            print(f"  {n:<12} {h:#08x}  {f[2] if f else '?'}")


def cmd_showmany(path):
    """Read many handlers in one call. Each input line: `ADDR [N] [label...]`
    (blank lines and `#` comments skipped). `path == '-'` reads from stdin."""
    src = sys.stdin if path == "-" else open(path)
    for line in src:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        addr = parts[0]
        n = int(parts[1]) if len(parts) > 1 and parts[1].lstrip("-").isdigit() else 40
        label = " ".join(parts[2:]) if len(parts) > 2 else ""
        print(f"\n===== {addr} {('· ' + label) if label else ''}=====")
        cmd_show(addr, n)


def cmd_handler(name):
    data = _data()
    funcs = _funcs()
    u = name.upper()
    # Authoritative first: the dispatch table pins name -> handler exactly.
    disp = _dispatch_table(data).get(u)
    if disp:
        print(f"# {u!r} handler (from dispatch table — authoritative):")
        for h in disp:
            f = _func_at(h, funcs)
            print(f"  {h:#08x}  {f[2] if f else '?'}")
        return
    print(f"# {u!r} not in dispatch table — heuristic (operator/special form?):")
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
    elif c == "showmany":
        cmd_showmany(arg)
    elif c == "dispatch":
        cmd_dispatch(arg)
    elif c == "handler":
        cmd_handler(arg)
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
