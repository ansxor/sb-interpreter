#!/usr/bin/env python3
"""GTRI fill rasterizer — working models vs the harvested skew masks (bd:sb-interpreter-j4l).

Two models that each reproduce ~97% of out/gtri_fill_skew.tsv:
  - band_model():    float center-band  [y-0.5, y+0.5) ; exact on skewA/B, 1px on skewC,
                     under-fills steep edges (skewD/E).
  - dda_union():     integer Bresenham edge runs unioned (the literal HW structure from
                     FUN_00155f8c/FUN_0015cab0) ; reproduces steep fattening but ~1px phase
                     off on shallow edges (seed `err0` not yet pinned — see gtri_fill_RE.md).

Run:  python3 harness/harvest/gtri_fill_model.py
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
TSV = os.path.join(HERE, "out", "gtri_fill_skew.tsv")

# (vertices, bounding box x,y,w,h) — must match gtri_fill_skew_cases.txt
CASES = {
    "skewA": ([(0, 0), (20, 0), (20, 6)], (0, 0, 21, 7)),
    "skewB": ([(0, 0), (13, 0), (0, 7)], (0, 0, 14, 8)),
    "skewC": ([(10, 10), (40, 13), (10, 20)], (10, 10, 31, 11)),
    "skewD": ([(3, 0), (9, 20), (0, 12)], (0, 0, 10, 21)),
    "skewE": ([(0, 0), (8, 3), (3, 9)], (0, 0, 9, 10)),
}


def load_rows():
    masks = {}
    for line in open(TSV):
        line = line.rstrip("\n")
        if not line.strip():
            continue
        name, vals = line.split("\t", 1)
        arr = [1 if v != "0" else 0 for v in vals.split()]
        bx, by, w, h = CASES[name][1]
        rows = {}
        i = 0
        for dy in range(h):
            xs = [bx + dx for dx in range(w) if arr[i + dx]]
            i += w
            if xs:
                rows[by + dy] = (min(xs), max(xs))
        masks[name] = rows
    return masks


def band_model(verts):
    ys = [p[1] for p in verts]
    edges = [(verts[0], verts[1]), (verts[1], verts[2]), (verts[2], verts[0])]
    out = {}
    for y in range(min(ys), max(ys) + 1):
        bandlo, bandhi = y - 0.5, y + 0.5
        mins, maxs = [], []
        for (xa, ya), (xb, yb) in edges:
            lo_y, hi_y = min(ya, yb), max(ya, yb)
            o0, o1 = max(bandlo, lo_y), min(bandhi, hi_y)
            if o0 > o1:
                continue
            if ya == yb:
                mins.append(min(xa, xb)); maxs.append(max(xa, xb)); continue
            x0 = xa + (o0 - ya) / (yb - ya) * (xb - xa)
            x1 = xa + (o1 - ya) / (yb - ya) * (xb - xa)
            xlo, xhi = min(x0, x1), max(x0, x1)
            openbot = (o1 == bandhi and o1 < hi_y and xa != xb)  # half-open bottom
            if openbot:
                if x1 == xhi:
                    xhi -= 1e-9
                if x1 == xlo:
                    xlo += 1e-9
            mins.append(xlo); maxs.append(xhi)
        if mins:
            out[y] = (math.ceil(min(mins)), math.floor(max(maxs)))
    return out


def _edge_run(x0, y0, x1, y1):
    out = {}
    adx, ady = abs(x1 - x0), abs(y1 - y0)
    r5, r4 = 2 * adx, 2 * ady
    err = -adx if adx > ady else ady  # FUN_00155f8c seed (OPEN QUESTION: ~1px out of phase)
    if x0 == x1:
        for y in range(y0, y1 + 1):
            out[y] = (x0, x0)
        return out
    if y1 < y0:
        return out
    x, y = x0, y0
    if x0 < x1:
        while y <= y1:
            err += r5; old = x
            if err >= r4:
                while True:
                    x += 1; err -= r4
                    if not (x <= x1 and r4 <= err):
                        break
            out[y] = (old, x if old == x else x - 1); y += 1
    else:
        while y <= y1:
            err += r5; old = x
            if err >= r4:
                while True:
                    x -= 1; err -= r4
                    if not (x >= x1 and r4 <= err):
                        break
            out[y] = (x if old == x else x + 1, old); y += 1
    return out


def dda_union(verts, flip_h=240):
    v = [(x, flip_h - 1 - yv) for x, yv in verts] if flip_h else list(verts)
    v.sort(key=lambda p: (p[1], p[0]))
    (x0, y0), (x1, y1), (x2, y2) = v
    table = {}
    for a, b in [((x0, y0), (x2, y2)), ((x0, y0), (x1, y1)), ((x1, y1), (x2, y2))]:
        for yy, (lo, hi) in _edge_run(a[0], a[1], b[0], b[1]).items():
            if yy in table:
                pl, ph = table[yy]; table[yy] = (min(pl, lo), max(ph, hi))
            else:
                table[yy] = (lo, hi)
    if flip_h:
        return {flip_h - 1 - yy: val for yy, val in table.items()}
    return table


def score(model):
    masks = load_rows()
    total = 0
    for name, (verts, _) in CASES.items():
        rows = masks[name]
        pred = model(verts)
        miss = 0
        for y in set(rows) | set(pred):
            t = rows.get(y, (10 ** 9, -10 ** 9))
            p = pred.get(y, (10 ** 9, -10 ** 9))
            for x in range(min(t[0], p[0]), max(t[1], p[1]) + 1):
                if (t[0] <= x <= t[1]) != (p[0] <= x <= p[1]):
                    miss += 1
        total += miss
        print(f"  {name}: {miss} px wrong")
    print(f"  TOTAL: {total} px wrong")
    return total


if __name__ == "__main__":
    print("band_model (float center-band):")
    score(band_model)
    print("dda_union (integer Bresenham, Y-flip):")
    score(lambda v: dda_union(v, flip_h=240))
