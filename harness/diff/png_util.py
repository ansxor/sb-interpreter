#!/usr/bin/env python3
"""Minimal PNG decode + (re)encode for the M2-T5 graphics golden pixel-diff.

The committed goldens are ordinary 8-bit RGBA PNGs (color type 6, no interlace) — either
harvested from real SB 3.6.0 (an oracle GRP capture decoded by `sb_grp.py`, O-T6; or a
composite screenshot via `run_case.py composite`, O-T6 screenshot path) or, until that
harvest lands, rendered by `sb-run --grp` (oracle-pending; tracked in beads — `bd ready`). The
renderer (`sb_render::png`) emits *uncompressed* (stored-deflate) PNGs, so `decode_rgba`
must accept both: it just runs the IDAT through `zlib.decompress`, which transparently
handles stored, fixed- and dynamic-Huffman blocks alike. `encode_rgba` re-compresses
(level 9) so committed goldens stay small regardless of how they were produced.

stdlib only (zlib + struct) — no Pillow, so the deterministic gate has no third-party dep.
"""
import struct
import zlib


def decode_rgba(data):
    """PNG bytes -> (width, height, rgba8888 bytes). Supports 8-bit RGBA, filters 0-4."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos = 8
    width = height = None
    idat = bytearray()
    while pos < len(data):
        (length,) = struct.unpack_from(">I", data, pos)
        typ = data[pos + 4 : pos + 8]
        chunk = data[pos + 8 : pos + 8 + length]
        if typ == b"IHDR":
            width, height, depth, color = struct.unpack_from(">IIBB", chunk, 0)
            if depth != 8 or color != 6:
                raise ValueError(f"unsupported PNG (depth={depth} color={color}); need 8-bit RGBA")
        elif typ == b"IDAT":
            idat += chunk
        elif typ == b"IEND":
            break
        pos += 12 + length  # length(4) + type(4) + data + crc(4)

    raw = zlib.decompress(bytes(idat))
    stride = width * 4
    out = bytearray(width * height * 4)
    prev = bytearray(stride)
    p = 0
    for y in range(height):
        ftype = raw[p]
        p += 1
        line = bytearray(raw[p : p + stride])
        p += stride
        _unfilter(line, prev, ftype, 4)
        out[y * stride : (y + 1) * stride] = line
        prev = line
    return width, height, bytes(out)


def _unfilter(line, prev, ftype, bpp):
    """Reverse a single PNG scanline filter in place (0=None,1=Sub,2=Up,3=Average,4=Paeth)."""
    if ftype == 0:
        return
    for i in range(len(line)):
        a = line[i - bpp] if i >= bpp else 0
        b = prev[i]
        c = prev[i - bpp] if i >= bpp else 0
        x = line[i]
        if ftype == 1:
            line[i] = (x + a) & 0xFF
        elif ftype == 2:
            line[i] = (x + b) & 0xFF
        elif ftype == 3:
            line[i] = (x + ((a + b) >> 1)) & 0xFF
        elif ftype == 4:
            line[i] = (x + _paeth(a, b, c)) & 0xFF
        else:
            raise ValueError(f"bad PNG filter {ftype}")


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def encode_rgba(width, height, rgba):
    """8-bit RGBA bytes -> compressed PNG bytes (color type 6, filter 0 per row, level 9)."""

    def chunk(typ, body):
        return struct.pack(">I", len(body)) + typ + body + struct.pack(
            ">I", zlib.crc32(typ + body) & 0xFFFFFFFF
        )

    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw += rgba[y * stride : (y + 1) * stride]
    out = b"\x89PNG\r\n\x1a\n"
    out += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    out += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    out += chunk(b"IEND", b"")
    return out


def diff_rgba(a, b):
    """Compare two (w,h,rgba) decoded images. Returns (mismatched_pixels, total, first_xy).

    Differing dimensions count as a total mismatch (returns -1 pixels). `first_xy` is the
    (x, y) of the first differing pixel, or None when they match.
    """
    aw, ah, ap = a
    bw, bh, bp = b
    if (aw, ah) != (bw, bh):
        return -1, aw * ah, None
    bad = 0
    first = None
    for i in range(aw * ah):
        o = i * 4
        if ap[o : o + 4] != bp[o : o + 4]:
            bad += 1
            if first is None:
                first = (i % aw, i // aw)
    return bad, aw * ah, first
