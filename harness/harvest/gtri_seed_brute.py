#!/usr/bin/env python3
"""Brute-force the GTRI edge-DDA seed/flip/endpoint conventions vs ALL masks (bd:sb-interpreter-j4l).

Loads the 5 line masks (gtri_fill_line.tsv) + 5 skew masks (gtri_fill_skew.tsv) and searches
the small parameter space of the integer-Bresenham union model for an EXACT (0 px) fit.
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))

LINE_CASES = {
    "lineA": ([(0, 0), (1, 9), (0, 9)], (0, 0, 2, 10)),
    "lineB": ([(0, 0), (9, 9), (0, 9)], (0, 0, 10, 10)),
    "lineC": ([(0, 0), (20, 3), (0, 3)], (0, 0, 21, 4)),
    "lineD": ([(0, 0), (3, 20), (0, 20)], (0, 0, 4, 21)),
    "lineE": ([(0, 0), (9, 4), (0, 4)], (0, 0, 10, 5)),
}
SKEW_CASES = {
    "skewA": ([(0, 0), (20, 0), (20, 6)], (0, 0, 21, 7)),
    "skewB": ([(0, 0), (13, 0), (0, 7)], (0, 0, 14, 8)),
    "skewC": ([(10, 10), (40, 13), (10, 20)], (10, 10, 31, 11)),
    "skewD": ([(3, 0), (9, 20), (0, 12)], (0, 0, 10, 21)),
    "skewE": ([(0, 0), (8, 3), (3, 9)], (0, 0, 9, 10)),
}


def load(tsv, cases):
    masks = {}
    for line in open(os.path.join(HERE, "out", tsv)):
        line = line.rstrip("\n")
        if not line.strip():
            continue
        name, vals = line.split("\t", 1)
        arr = [1 if v != "0" else 0 for v in vals.split()]
        bx, by, w, h = cases[name][1]
        rows = {}
        i = 0
        for dy in range(h):
            xs = [bx + dx for dx in range(w) if arr[i + dx]]
            i += w
            if xs:
                rows[by + dy] = (min(xs), max(xs))
        masks[name] = rows
    return masks


def edge_run(x0, y0, x1, y1, seed_kind, end_conv):
    """Integer Bresenham edge run table; y0<=y1 required. Returns {y:(lo,hi)}."""
    out = {}
    adx, ady = abs(x1 - x0), abs(y1 - y0)
    r5, r4 = 2 * adx, 2 * ady
    # seed variants
    if seed_kind == 0:
        err = -adx if adx > ady else ady
    elif seed_kind == 1:
        err = -adx if adx > ady else -ady
    elif seed_kind == 2:
        err = adx if adx > ady else ady
    elif seed_kind == 3:
        err = 0
    elif seed_kind == 4:
        err = -ady if adx > ady else adx  # swapped
    elif seed_kind == 5:
        err = (-adx if adx > ady else ady) + ady
    elif seed_kind == 6:
        err = (-adx if adx > ady else ady) - ady
    if x0 == x1:
        for y in range(y0, y1 + 1):
            out[y] = (x0, x0)
        return out
    if y1 < y0:
        return out
    x = x0
    inc = 1 if x0 < x1 else -1
    for y in range(y0, y1 + 1):
        err += r5
        old = x
        if err >= r4:
            while True:
                x += inc
                err -= r4
                if not (((inc > 0 and x <= x1) or (inc < 0 and x >= x1)) and r4 <= err):
                    break
        new = x
        if old == new:
            lo = hi = old
        else:
            # endpoint convention for the trailing pixel
            if inc > 0:
                lo, hi = old, (new if end_conv == 1 else new - 1)
                if end_conv == 2:
                    lo, hi = old + 1, new  # drop leading instead
            else:
                lo, hi = (new if end_conv == 1 else new + 1), old
                if end_conv == 2:
                    lo, hi = new, old - 1
        out[y] = (min(lo, hi), max(lo, hi))
    return out


def dda_union(verts, flip_h, seed_kind, end_conv):
    v = [(x, (flip_h - 1 - yv) if flip_h else yv) for x, yv in verts]
    v.sort(key=lambda p: (p[1], p[0]))
    (x0, y0), (x1, y1), (x2, y2) = v
    table = {}
    for a, b in [((x0, y0), (x2, y2)), ((x0, y0), (x1, y1)), ((x1, y1), (x2, y2))]:
        for yy, (lo, hi) in edge_run(a[0], a[1], b[0], b[1], seed_kind, end_conv).items():
            if yy in table:
                pl, ph = table[yy]
                table[yy] = (min(pl, lo), max(ph, hi))
            else:
                table[yy] = (lo, hi)
    if flip_h:
        return {flip_h - 1 - yy: val for yy, val in table.items()}
    return table


def score_case(rows, pred):
    miss = 0
    for y in set(rows) | set(pred):
        t = rows.get(y, (10 ** 9, -10 ** 9))
        p = pred.get(y, (10 ** 9, -10 ** 9))
        for x in range(min(t[0], p[0]), max(t[1], p[1]) + 1):
            if (t[0] <= x <= t[1]) != (p[0] <= x <= p[1]):
                miss += 1
    return miss


def main():
    lm = load("gtri_fill_line.tsv", LINE_CASES)
    sm = load("gtri_fill_skew.tsv", SKEW_CASES)
    allmasks = {**lm, **sm}
    allcases = {**LINE_CASES, **SKEW_CASES}
    best = None
    for flip_h in [0, 240, 256, 512]:
        for seed_kind in range(7):
            for end_conv in [0, 1, 2]:
                total = 0
                per = {}
                for name, (verts, _) in allcases.items():
                    pred = dda_union(verts, flip_h, seed_kind, end_conv)
                    m = score_case(allmasks[name], pred)
                    per[name] = m
                    total += m
                if best is None or total < best[0]:
                    best = (total, flip_h, seed_kind, end_conv, dict(per))
    total, flip_h, seed_kind, end_conv, per = best
    print(f"BEST total={total}  flip_h={flip_h} seed_kind={seed_kind} end_conv={end_conv}")
    for k, v in per.items():
        print(f"    {k}: {v}")


if __name__ == "__main__":
    main()
