#!/usr/bin/env python3
"""Decode a SmileBASIC GRP file (graphics page saved with SAVE"GRP<n>:NAME") to RGBA pixels,
and write a PNG — no third-party deps (zlib + struct only).

GRP file layout (verified against real SB 3.6.0, GRP0 of a 512x512 page):
  extdata file = 80-byte header + body + 20-byte HMAC footer   (see sb_extdata.py)
  body = 28-byte internal header + raw pixel data:
    body[0:4]   magic  "PCBN"
    body[4:8]   version "0001"
    body[8:12]  type/flags (0x0003, 0x0002 as u16) — not needed for decode
    body[12:16] width  (u32 LE)  -> 512
    body[16:20] height (u32 LE)  -> 512
    body[20:28] (checksum/date-ish + zero) — ignored
    body[28:]   width*height pixels, 16-bit RGBA5551, little-endian, row-major, top-left origin
  RGBA5551 bit layout (MSB->LSB): R:5 G:5 B:5 A:1  (alpha is bit 0; 1=opaque, 0=transparent).
The whole 512x512 page is always saved (incl. the off-screen region beyond the visible 400x240).
"""
import struct
import sys
import zlib

HEADER, FOOTER, PCBN = 80, 20, 28


def decode_grp(file_bytes, expand="shift"):
    """GRP file bytes -> (width, height, rgba8888). `expand` controls 5->8-bit channel scaling:
    'shift' = v<<3 (matches sb-render's expand5 / SB's logical-color constants, e.g.
    #WHITE=&HFFF8F8F8); 'full' = v<<3 | v>>2 (uses the full 0..255 range)."""
    body = file_bytes[HEADER:-FOOTER]
    if body[:4] != b"PCBN":
        raise ValueError(f"not a PCBN GRP body (got {body[:8]!r})")
    w = struct.unpack_from("<I", body, 12)[0]
    h = struct.unpack_from("<I", body, 16)[0]
    pix = body[PCBN:]
    if w * h * 2 != len(pix):
        raise ValueError(f"dim/size mismatch: {w}x{h} -> expect {w*h*2} px bytes, got {len(pix)}")
    rgba = bytearray(w * h * 4)
    for i in range(w * h):
        v = pix[2 * i] | (pix[2 * i + 1] << 8)
        r, g, b, a = (v >> 11) & 31, (v >> 6) & 31, (v >> 1) & 31, v & 1
        if expand == "full":
            r, g, b = (r << 3) | (r >> 2), (g << 3) | (g >> 2), (b << 3) | (b >> 2)
        else:
            r, g, b = r << 3, g << 3, b << 3
        o = 4 * i
        rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3] = r, g, b, 255 if a else 0
    return w, h, bytes(rgba)


def crop(width, height, rgba, w2, h2):
    """Top-left crop of an RGBA buffer to w2 x h2 (e.g. 512x512 page -> visible 400x240)."""
    out = bytearray(w2 * h2 * 4)
    for y in range(h2):
        src = (y * width) * 4
        out[y * w2 * 4: (y + 1) * w2 * 4] = rgba[src: src + w2 * 4]
    return bytes(out)


def write_png(path, width, height, rgba):
    """Write 8-bit RGBA pixels as a PNG (color type 6, no interlace), stdlib only."""
    def chunk(typ, data):
        return (struct.pack(">I", len(data)) + typ + data
                + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF))

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)                       # filter type 0 (none) per scanline
        raw += rgba[y * stride: (y + 1) * stride]
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))
    return path


if __name__ == "__main__":
    # Decode a GRP file already on disk -> PNG. Usage: sb_grp.py GRPFILE OUT.png [VISW VISH]
    a = sys.argv[1:]
    if len(a) < 2:
        print("usage: sb_grp.py <grp-file> <out.png> [crop_w crop_h]")
        sys.exit(2)
    w, h, rgba = decode_grp(open(a[0], "rb").read())
    if len(a) >= 4:
        cw, ch = int(a[2]), int(a[3])
        rgba = crop(w, h, rgba, cw, ch)
        w, h = cw, ch
    write_png(a[1], w, h, rgba)
    print(f"wrote {a[1]} ({w}x{h})")
