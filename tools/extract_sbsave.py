#!/usr/bin/env python3
"""Extract the `sbsave/` scrape into an extdata-compatible test corpus.

`sbsave/` is a scrape of the smilebasicsource.com public server (taken before it
shut down): `sbsave/{sucess,error}/<KEY>/{body.bin,headers.json}`. `sucess/` holds
915 real downloads; `error/` holds failed fetches (404/private) and is ignored.

Each `body.bin` is a PETC server file. This tool decodes it and writes a faithful,
extdata-injectable tree under `harness/corpus/sbsave/files/<KEY>/{TXT,DAT}/<NAME>`
plus a committed `INDEX.json` manifest the conformance harness / Ralph loop reads to
pick test cases. Run `python3 tools/extract_sbsave.py` from the repo root.

──────────────────────────────────────────────────────────────────────────────
PETC server file format (reverse-engineered here; validated 915/915)
──────────────────────────────────────────────────────────────────────────────
Every file:  [ 80-byte SB3 header ][ payload ][ 20-byte SHA1 footer ]
  header 0x00 u16  version (1)
         0x02 u16  type: 0=TXT (program text), 1=DAT (PCBN binary), 2=PRJ (project)
         0x08 u32  payload size
         0x0C      created date: u16 year, u8 month/day/hour/min/sec
         0x14      author name (creator), then current author  (18-byte fields)
  footer        SHA1 of header+payload

type 0 (TXT):  payload = UTF-8 SmileBASIC source (uppercase kw, @LABEL, ?=PRINT).
type 1 (DAT):  payload = a PCBN0001 binary (GRP graphics / int|double|ushort arrays;
               no magic ⇒ raw int32 array, per osb SMILEBASIC/project.d loadDataFile).
type 2 (PRJ):  a project package bundling many internal files:
         0x50 u32  project total size
         0x54 u32  file count N
         0x58      directory: N × { u32 fullSize, char[16] name }   (20 bytes each)
         then      N standalone SB3 files (each its own 80+payload+20), concatenated
         then      the project's own 20-byte SHA1 footer
  Internal names carry a 1-char type prefix: 'T'→TXT, 'B'→DAT. The on-device resource
  name (what `LOAD "TXT:foo"` uses) is name[1:]; the prefix picks the folder.
"""
import argparse
import json
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "sbsave" / "sucess"
OUT = REPO / "harness" / "corpus" / "sbsave"
FILES = OUT / "files"

HDR = 80            # SB3 header length (0x50)
FOOT = 20           # SHA1 footer length (0x14)
TYPE_TXT, TYPE_DAT, TYPE_PRJ = 0, 1, 2


def safe_name(name):
    """Keep SmileBASIC's own filename charset; neutralize anything else for the host FS."""
    s = "".join(c if (c.isalnum() or c in "._-@") else "_" for c in name)
    return "UNNAMED" if s.strip(".") == "" else s  # guard ""/"."/".." (dir-traversal)


def split_prefixed(name):
    """Internal project name 'T3DSCLOSEGAME' / 'BSP' → ('TXT'|'DAT', 'resource name')."""
    if name[:1] == "T":
        return "TXT", name[1:]
    if name[:1] == "B":
        return "DAT", name[1:]
    return "DAT", name  # unprefixed: treat as data, keep verbatim


def parse_project(data):
    """Yield (folder, resource_name, payload_bytes) for each internal file of a PRJ."""
    count = struct.unpack_from("<I", data, 0x54)[0]
    diroff = 0x58
    entries = []
    for _ in range(count):
        size = struct.unpack_from("<I", data, diroff)[0]
        name = data[diroff + 4:diroff + 20].split(b"\0")[0].decode("latin1")
        entries.append((name, size))
        diroff += 20
    off = diroff
    for name, size in entries:
        blob = data[off:off + size]
        if len(blob) != size:
            raise ValueError(f"truncated internal file {name!r}")
        folder, resname = split_prefixed(name)
        yield folder, resname, blob[HDR:-FOOT]
        off += size
    if off != len(data) - FOOT:
        raise ValueError(f"project trailing mismatch: {off} != {len(data) - FOOT}")


def decode_files(body, public_name):
    """Normalize any PETC file to a list of (folder, resource_name, payload_bytes)."""
    ftype = struct.unpack_from("<H", body, 2)[0]
    if ftype == TYPE_PRJ:
        return list(parse_project(body))
    folder, resname = split_prefixed(public_name)
    if ftype == TYPE_TXT:
        folder = "TXT"
    elif ftype == TYPE_DAT:
        folder = "DAT"
    return [(folder, resname or safe_name(public_name), body[HDR:-FOOT])]


def iter_downloads():
    for d in sorted(SRC.iterdir()):
        body_f, hdr_f = d / "body.bin", d / "headers.json"
        if not body_f.exists():
            continue
        meta = json.loads(hdr_f.read_text()) if hdr_f.exists() else {}
        yield d.name, body_f.read_bytes(), meta


def text_preview(payload):
    """Decode a TXT payload to source; flag if it isn't clean UTF-8 (SB private glyphs)."""
    try:
        return payload.decode("utf-8"), "utf-8"
    except UnicodeDecodeError:
        return payload.decode("utf-8", "replace"), "sb-bytes"


def run(extract=True, limit=None):
    index = []
    n_prj = n_txt = n_dat = n_fail = 0
    for i, (key, body, meta) in enumerate(iter_downloads()):
        if limit and i >= limit:
            break
        if len(body) < HDR + FOOT:
            n_fail += 1
            continue
        public = meta.get("X-Petc-FileName", key)
        ftype = struct.unpack_from("<H", body, 2)[0]
        try:
            files = decode_files(body, public)
        except Exception as e:  # noqa: BLE001 — corpus is fixed; record & skip stragglers
            n_fail += 1
            print(f"  ! {key} ({public}): {e}", file=sys.stderr)
            continue
        n_prj += ftype == TYPE_PRJ
        members = []
        seen = {}
        for folder, resname, payload in files:
            fname = safe_name(resname)
            if (folder, fname) in seen:  # de-collide duplicate internal names
                seen[(folder, fname)] += 1
                fname = f"{fname}~{seen[(folder, fname)]}"
            else:
                seen[(folder, fname)] = 0
            is_txt = folder == "TXT"
            n_txt += is_txt
            n_dat += not is_txt
            enc = None
            if is_txt:
                _, enc = text_preview(payload)
            members.append({
                "folder": folder, "name": fname,
                "bytes": len(payload), "encoding": enc,
            })
            if extract:
                dest = FILES / key / folder / fname
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(payload)  # raw bytes: byte-exact for extdata injection
        index.append({
            "key": key,
            "public_name": public,
            "type": {0: "TXT", 1: "DAT", 2: "PRJ"}.get(ftype, ftype),
            "author": meta.get("X-Petc-Author"),
            "date": meta.get("X-Petc-Date"),
            "refcount": int(meta.get("X-Petc-RefCount", 0) or 0),
            "files": members,
        })

    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {
        "source": "smilebasicsource.com public server scrape (sbsave/sucess)",
        "downloads": len(index),
        "programs_txt": n_txt,
        "data_dat": n_dat,
        "projects": n_prj,
        "failed": n_fail,
        "entries": index,
    }
    (OUT / "INDEX.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False))
    print(f"downloads={len(index)} TXT={n_txt} DAT={n_dat} projects={n_prj} failed={n_fail}")
    print(f"manifest → {OUT / 'INDEX.json'}" + (f"  files → {FILES}/" if extract else ""))
    return manifest


def get(spec):
    """Print one resource to stdout: KEY (first TXT), KEY/NAME, or KEY/FOLDER/NAME."""
    parts = spec.split("/")
    key = parts[0]
    body = (SRC / key / "body.bin").read_bytes()
    meta_f = SRC / key / "headers.json"
    public = json.loads(meta_f.read_text()).get("X-Petc-FileName", key) if meta_f.exists() else key
    files = decode_files(body, public)
    want_folder = parts[1].upper() if len(parts) == 3 else None
    want_name = parts[-1] if len(parts) >= 2 else None
    for folder, resname, payload in files:
        if want_name and safe_name(resname) != safe_name(want_name):
            continue
        if want_folder and folder != want_folder:
            continue
        if folder == "TXT" or not want_name:
            sys.stdout.buffer.write(payload if folder == "TXT" else payload)
            return
    print(f"not found: {spec}", file=sys.stderr)
    sys.exit(1)


def main():
    ap = argparse.ArgumentParser(description="Extract sbsave/ into an extdata-compatible corpus.")
    ap.add_argument("--manifest-only", action="store_true", help="rewrite INDEX.json without dumping files")
    ap.add_argument("--get", metavar="KEY[/NAME]", help="print one program/resource to stdout (cheap test-case fetch)")
    ap.add_argument("--limit", type=int, help="process only the first N downloads (debug)")
    args = ap.parse_args()
    if args.get:
        get(args.get)
    else:
        run(extract=not args.manifest_only, limit=args.limit)


if __name__ == "__main__":
    main()
