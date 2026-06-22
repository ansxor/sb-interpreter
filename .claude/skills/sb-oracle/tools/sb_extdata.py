#!/usr/bin/env python3
"""Read (and tentatively write) SmileBASIC files in Azahar's extdata on disk.

Format (cracked): 80-byte header + UTF-8 source + 20-byte footer.
  header[0x08:0x0C] = source length (LE u32); header[0x0C:] = save timestamp.
  body[80:80+len]   = source.
  footer            = 20-byte HMAC-SHA1 (keyed). READING ignores it. WRITING a loadable
                      file needs SB's HMAC key (not yet recovered) — write_program() emits
                      a structurally-correct file with a placeholder footer that SB will
                      likely REJECT on load until the key is found. See HARVEST_QUEUE.md.
"""
import glob
import os
import struct
import sys

HOME = os.path.expanduser("~")
EXTDATA_GLOB = (f"{HOME}/Library/Application Support/Azahar/sdmc/Nintendo 3DS/"
                f"*/*/extdata/00000000/000016DE/user")
HEADER = 80
FOOTER = 20


def _user_dir():
    hits = glob.glob(EXTDATA_GLOB)
    if not hits:
        raise FileNotFoundError("SmileBASIC extdata user dir not found (is SB installed in Azahar?)")
    return hits[0]


def _path(name):
    # Citra/Azahar store files under user/###/<NAME>.
    return os.path.join(_user_dir(), "###", name)


def list_files():
    base = os.path.join(_user_dir(), "###")
    return sorted(f for f in os.listdir(base)) if os.path.isdir(base) else []


def read_program(name):
    """Return the UTF-8 source of an SB file (footer-agnostic)."""
    data = open(_path(name), "rb").read()
    n = struct.unpack_from("<I", data, 8)[0]
    body = data[HEADER:HEADER + n] if n else data[HEADER:-FOOTER]
    return body.decode("utf-8", "replace")


def read_raw(name):
    return open(_path(name), "rb").read()


# In-SB names map to on-disk names by a type-char prefix: TXT -> "T".
# e.g. SB `SAVE"TXT:O"` writes on-disk file "TO"; `SAVE"T"` (TXT default) -> "TT".
TYPE_PREFIX = {"TXT": "T", "DAT": "B", "PRG": "P"}


def read_result(sb_name, ftype="TXT"):
    """Read a file by its in-SB name + type (TXT/DAT/PRG). Returns the UTF-8 source."""
    return read_program(TYPE_PREFIX.get(ftype, "T") + sb_name)


def result_mtime(sb_name, ftype="TXT"):
    try:
        return os.path.getmtime(_path(TYPE_PREFIX.get(ftype, "T") + sb_name))
    except OSError:
        return None


def write_program(name, source: str):
    """Write an SB file (header+source+placeholder footer). NOTE: SB may reject this on
    load until the HMAC-SHA1 key is recovered. Provided for experimentation."""
    src = source.encode("utf-8")
    hdr = bytearray(HEADER)
    struct.pack_into("<I", hdr, 0, 1)          # version/magic (matches observed 0x01)
    struct.pack_into("<I", hdr, 4, 0x00010000) # flags (observed)
    struct.pack_into("<I", hdr, 8, len(src))   # source length
    foot = b"\x00" * FOOTER                     # TODO: HMAC-SHA1(key, hdr+src)
    open(_path(name), "wb").write(bytes(hdr) + src + foot)
    return _path(name)


def main():
    a = sys.argv[1:]
    if not a:
        print("usage: list | read NAME | raw NAME | write NAME SOURCE"); return
    if a[0] == "list":
        print("\n".join(list_files()) or "(no files)")
    elif a[0] == "read":
        print(read_program(a[1]))
    elif a[0] == "raw":
        print(read_raw(a[1]).hex())
    elif a[0] == "write":
        print("wrote:", write_program(a[1], a[2]))


if __name__ == "__main__":
    main()
