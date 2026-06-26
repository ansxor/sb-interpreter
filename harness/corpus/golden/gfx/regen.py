#!/usr/bin/env python3
"""Seed a FALLBACK graphics golden from sb-core's own renderer (NOT ground truth).

For each `<name>.sb3` drawing program in this directory, run it through `sb-run --grp` (the
headless renderer dumps the GRP display page, visible 400×240 crop, as an uncompressed PNG),
then re-compress it to `<name>.png`.

PROVENANCE — read before using: the committed goldens here are **hw_verified oracle GRP
captures** of real SB 3.6.0 (`run_case.py grp` → `sb_grp.py` decode → 400×240 shift crop,
O-T6). A golden produced by THIS script is instead sb-core diffed against sb-core — a pure
regression pin, NOT device truth. So this tool is only for SEEDING a brand-new case's
placeholder when the oracle is unavailable: it **refuses to overwrite an existing golden**
unless you pass `--force` (which you should not do to an oracle-harvested one). To add a real
golden, capture it on the oracle and drop the PNG in directly — the same `replay.py` diff
then applies, no regen needed. (Composite goldens — sprites+BG+backdrop — live in
`golden/composite/` via `run_case.py composite`; oracle-pending CI gate tracked in beads.)

Usage:  python3 regen.py NAME ...           # seed only-missing goldens for the named stems
        python3 regen.py                     # seed every program that has no golden yet
        python3 regen.py --force NAME ...    # overwrite (clobbers oracle goldens — careful)
"""
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / "harness" / "diff"))
from png_util import decode_rgba, encode_rgba  # noqa: E402

SB_RUN = ROOT / "target" / "debug" / "sb-run"


def regen(stem, force):
    prog = HERE / f"{stem}.sb3"
    if not prog.exists():
        print(f"  ! no program {prog.name}")
        return False
    out = HERE / f"{stem}.png"
    if out.exists() and not force:
        print(f"  - {stem}.png exists (oracle golden?) — skipping; --force to overwrite")
        return True
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tf:
        tmp = Path(tf.name)
    try:
        proc = subprocess.run(
            [str(SB_RUN), "--grp", str(tmp), str(prog)], capture_output=True, text=True
        )
        if proc.returncode not in (0, 1):  # 1 = SB error, page still drawn
            print(f"  ! {stem}: sb-run exit {proc.returncode}: {proc.stderr.strip()}")
            return False
        w, h, rgba = decode_rgba(tmp.read_bytes())
        out.write_bytes(encode_rgba(w, h, rgba))
        print(f"  {stem}.png  {w}x{h}  {out.stat().st_size} bytes")
        return True
    finally:
        tmp.unlink(missing_ok=True)


def main():
    if not SB_RUN.exists():
        print(f"build the runner first: cargo build -p sb-platform-native --bin sb-run")
        return 2
    argv = sys.argv[1:]
    force = "--force" in argv
    stems = [a for a in argv if a != "--force"] or sorted(p.stem for p in HERE.glob("*.sb3"))
    print(f"seeding {len(stems)} fallback golden(s) via {SB_RUN.name} (force={force}):")
    ok = all(regen(s, force) for s in stems)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
