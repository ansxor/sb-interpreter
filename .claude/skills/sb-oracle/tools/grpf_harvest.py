#!/usr/bin/env python3
"""1ip harvest: save GRPF: (font page) + GRP0: on real SB 3.6.0, read each PCBN body header
off disk, dump width/height/type-tag bytes so we can confirm GRPF layout vs the GRP0 assumption.

GRPF is the font/sprite-sheet page (page index -1 in GSAVE). We don't draw anything special;
SAVE captures the current page contents. We GFILL GRP0 so it has known content, but GRPF is
saved as-is (the resident font sheet)."""
import sys, time, struct
import run_case as R
import sb_extdata as X


def save_and_read(save_target, sb_name):
    """Run a program that SAVEs `save_target` to `sb_name`, return the raw on-disk body bytes
    (header+body+footer stripped to just the PCBN body)."""
    # GRP / GRPF both live under the B (data) on-disk prefix.
    onpath = X.TYPE_PREFIX["GRP"] + sb_name
    src = f'SAVE"{save_target}:{sb_name}"\n'
    X.write_file("P", src, "TXT")
    R.W.raise_window()
    time.sleep(0.4)
    R.W.confirm_dialogs()
    R._delete_result(sb_name, "GRP")
    R._clean()
    R._load_prog()
    R._run_prog()
    import os
    p = R._result_path(sb_name, "GRP")
    if not os.path.exists(p):
        raise TimeoutError(f"no file for {save_target}:{sb_name} at {p}")
    full = open(p, "rb").read()
    body = full[0x50:-0x14]  # strip 80-byte header + 20-byte HMAC footer
    return full, body


def dump(label, full, body):
    print(f"=== {label} ===")
    print(f"file total bytes: {len(full)}")
    hdr = full[:0x50]
    print(f"header[0x00:0x08] type marker: {hdr[:8].hex(' ')}")
    print(f"header[0x08:0x0C] body len   : {struct.unpack('<I', hdr[8:12])[0]} (0x{struct.unpack('<I', hdr[8:12])[0]:08x})")
    print(f"body len actual            : {len(body)}")
    print(f"PCBN body[0x00:0x1C] (28B) : {body[:0x1C].hex(' ')}")
    print(f"  magic     : {body[0:4]!r}")
    print(f"  version   : {body[4:8]!r}")
    print(f"  type/flags: {body[8:12].hex(' ')}  u16@8={struct.unpack('<H', body[8:10])[0]} u16@10={struct.unpack('<H', body[10:12])[0]}")
    w = struct.unpack('<I', body[12:16])[0]
    h = struct.unpack('<I', body[16:20])[0]
    print(f"  width @0x0C: {w}")
    print(f"  height@0x10: {h}")
    print(f"  rest @0x14:8: {body[20:28].hex(' ')}")
    print(f"  pixel bytes (len-28): {len(body)-28}  expected w*h*2={w*h*2}")
    print()


if __name__ == "__main__":
    print("readying oracle...")
    # ensure ready
    full0, body0 = save_and_read("GRP0", "ZQGRP0")
    dump("GRP0:ZQGRP0", full0, body0)
    fullf, bodyf = save_and_read("GRPF", "ZQGRPF")
    dump("GRPF:ZQGRPF", fullf, bodyf)
