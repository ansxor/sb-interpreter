#!/usr/bin/env python3
"""Pin the GTRI edge-DDA seed phase from the thin single-edge line masks (bd:sb-interpreter-j4l).

Each lineX = GTRI 0,0,W,H,0,H : vertical left edge x=0, hypotenuse (0,0)->(W,H).
Span per scanline = [0, rightX(y)] so rightX(y) directly reveals the boundary rule for slope W/H.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
TSV = os.path.join(HERE, "out", "gtri_fill_line.tsv")

# name -> (W, H)  (box is (W+1)x(H+1) @ (0,0))
LINES = {"lineA": (1, 9), "lineB": (9, 9), "lineC": (20, 3), "lineD": (3, 20), "lineE": (9, 4)}


def load():
    masks = {}
    for line in open(TSV):
        line = line.rstrip("\n")
        if not line.strip():
            continue
        name, vals = line.split("\t", 1)
        W, H = LINES[name]
        w = W + 1
        arr = [1 if v != "0" else 0 for v in vals.split()]
        rows = {}
        i = 0
        for dy in range(H + 1):
            xs = [dx for dx in range(w) if arr[i + dx]]
            i += w
            rows[dy] = (min(xs), max(xs)) if xs else None
        masks[name] = rows
    return masks


def show_truth():
    masks = load()
    for name, (W, H) in LINES.items():
        rows = masks[name]
        rx = [rows[y][1] if rows[y] else None for y in range(H + 1)]
        print(f"  {name} (W={W},H={H}, slope {W}/{H}): rightX(y)= {rx}")
    return masks


if __name__ == "__main__":
    masks = show_truth()
