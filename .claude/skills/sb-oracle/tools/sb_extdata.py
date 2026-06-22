#!/usr/bin/env python3
"""Read and write SmileBASIC files in Azahar's extdata on disk.

Format (fully cracked, validated against real SB-saved files):
  80-byte header + UTF-8 body + 20-byte footer.
  header = type-marker(8) + body-length(LE u32) + date(DF 07 0A 0F) + zeros -> 80 bytes.
  footer = HMAC-SHA1(KEY, header + body)   [20 bytes]
  On-disk name = type-prefix + in-SB name  (TXT->"T", DAT/GRP->"B").
Source of truth for the header markers, prefixes, and HMAC key: nnn1590/lpp-3ds-sbfm
(romfs/index.lua, the SmileBASIC File Manager). Programs are TXT files, so a program "P"
is on-disk "TP" and loads via LOAD"PRG0:P".
"""
import glob
import hashlib
import hmac
import os
import struct
import sys

HOME = os.path.expanduser("~")
EXTDATA_GLOB = (f"{HOME}/Library/Application Support/Azahar/sdmc/Nintendo 3DS/"
                f"*/*/extdata/00000000/000016DE/user")
HEADER = 80
FOOTER = 20

# SmileBASIC's file-integrity HMAC-SHA1 key (from lpp-3ds-sbfm).
HMAC_KEY = b'nqmby+e9S?{%U*-V]51n%^xZMk8>b{?x]&?(NmmV[,g85:%6Sqd"\'U")/8u77UL2'
DATE = bytes([0xDF, 0x07, 0x0A, 0x0F])  # fixed save date used by SBFM
TYPE_MARKER = {
    "TXT": bytes([0x01, 0, 0, 0, 0, 0, 0x01, 0]),
    "DAT": bytes([0x01, 0, 0x01, 0, 0, 0, 0, 0]),
    "GRP": bytes([0x01, 0, 0x01, 0, 0, 0, 0x02, 0]),
}
TYPE_PREFIX = {"TXT": "T", "DAT": "B", "GRP": "B"}


def _user_dir():
    hits = glob.glob(EXTDATA_GLOB)
    if not hits:
        raise FileNotFoundError("SmileBASIC extdata user dir not found (is SB installed in Azahar?)")
    return hits[0]


def _path(name):
    return os.path.join(_user_dir(), "###", name)  # Azahar stores files under user/###/


def list_files():
    base = os.path.join(_user_dir(), "###")
    return sorted(os.listdir(base)) if os.path.isdir(base) else []


def read_program(name):
    """Return the UTF-8 source of an on-disk SB file (footer-agnostic)."""
    data = open(_path(name), "rb").read()
    n = struct.unpack_from("<I", data, 8)[0]
    body = data[HEADER:HEADER + n] if n else data[HEADER:-FOOTER]
    return body.decode("utf-8", "replace")


def read_raw(name):
    return open(_path(name), "rb").read()


def read_result(sb_name, ftype="TXT"):
    """Read a file by its in-SB name + type. Returns the UTF-8 source."""
    return read_program(TYPE_PREFIX[ftype] + sb_name)


def result_mtime(sb_name, ftype="TXT"):
    try:
        return os.path.getmtime(_path(TYPE_PREFIX[ftype] + sb_name))
    except OSError:
        return None


def build_file(source: str, ftype="TXT"):
    """Build the exact on-disk bytes for an SB file (valid header + HMAC footer)."""
    body = source.encode("utf-8")
    header = TYPE_MARKER[ftype] + struct.pack("<I", len(body)) + DATE + b"\x00" * 64
    assert len(header) == HEADER, len(header)
    footer = hmac.new(HMAC_KEY, header + body, hashlib.sha1).digest()
    return header + body + footer


def write_file(sb_name, source: str, ftype="TXT"):
    """Write a valid SB file by in-SB name + type (on-disk = prefix+name). SB accepts it
    (correct HMAC). For a program loadable via LOAD"PRG0:<sb_name>", use ftype='TXT'."""
    path = _path(TYPE_PREFIX[ftype] + sb_name)
    open(path, "wb").write(build_file(source, ftype))
    return path


def main():
    a = sys.argv[1:]
    if not a:
        print("usage: list | read DISKNAME | raw DISKNAME | write SBNAME SOURCE [TXT|DAT|GRP]")
        return
    if a[0] == "list":
        print("\n".join(list_files()) or "(no files)")
    elif a[0] == "read":
        print(read_program(a[1]))
    elif a[0] == "raw":
        print(read_raw(a[1]).hex())
    elif a[0] == "write":
        print("wrote:", write_file(a[1], a[2], a[3] if len(a) > 3 else "TXT"))


if __name__ == "__main__":
    main()
