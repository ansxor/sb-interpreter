#!/usr/bin/env python3
"""Faithful port of SB3.6.0 GTRI fill (FUN_00194120 + FUN_00155f8c init + FUN_0015cab0 union).
KEY: the DDA is run TRANSPOSED — table indexed by X (columns), storing Y' ranges; fill writes
vertical runs. RUN3 simulated it per-scanline (horizontal) and failed. Test against masks."""

def dda_init(x0, y0, x1, y1, table):
    # arg order (value0=Y', index0=X, value1, index1). loop var = index (X). assume index incr.
    r0, r2 = x0, x1          # value (Y') start/end
    r1, r3 = y0, y1          # index (X) start/end
    dV = abs(r2 - r0); dI = abs(r3 - r1)
    err = -dV if dV > dI else dI
    twodV = 2 * dV; twodI = 2 * dI
    if r0 == r2:             # constant value (horizontal screen edge)
        val = r0
        for i in range(max(0, r1), min(r3, 511) + 1):
            table[i] = (val, val)
        return
    if r3 < r1:
        return
    i = r1
    if r0 < r2:             # value increasing
        while i <= r3:
            err += twodV; old = r0
            if err >= twodI:
                while True:
                    r0 += 1; err -= twodI
                    if not (r0 <= r2 and twodI <= err):
                        break
            new = r0
            if i < 512:
                table[i] = (old, old if old == new else new - 1)
            i += 1
    else:                  # value decreasing
        while i <= r3:
            err += twodV; old = r0
            if err >= twodI:
                while True:
                    r0 -= 1; err -= twodI
                    if not (r2 <= r0 and twodI <= err):
                        break
            new = r0
            if i < 512:
                table[i] = (old if old == new else new + 1, old)
            i += 1

def dda_union(x0, y0, x1, y1, table):
    r0, r2 = x0, x1
    r1, r3 = y0, y1
    dV = abs(r2 - r0); dI = abs(r3 - r1)
    err = -dV if dV > dI else dI
    twodV = 2 * dV; twodI = 2 * dI
    if r0 == r2:
        val = r0
        for i in range(max(0, r1), min(r3, 511) + 1):
            if i in table:
                lo, hi = table[i]; table[i] = (min(lo, val), max(hi, val))
            else:
                table[i] = (val, val)
        return
    if r3 < r1:
        return
    i = r1
    if r0 < r2:
        while i <= r3:
            err += twodV; old = r0
            if err >= twodI:
                while True:
                    r0 += 1; err -= twodI
                    if not (r0 <= r2 and twodI <= err):
                        break
            new = r0
            if i < 512:
                cand_hi = new if old == new else new - 1
                lo, hi = table.get(i, (old, cand_hi))
                table[i] = (min(lo, old), max(hi, cand_hi))
            i += 1
    else:
        while i <= r3:
            err += twodV; old = r0
            if err >= twodI:
                while True:
                    r0 -= 1; err -= twodI
                    if not (r2 <= r0 and twodI <= err):
                        break
            new = r0
            if i < 512:
                cand_lo = new if old == new else new + 1
                lo, hi = table.get(i, (cand_lo, old))
                table[i] = (min(lo, cand_lo), max(hi, old))
            i += 1

def gtri_fill(v0, v1, v2, H):
    # flip y
    verts = [(x, H - y - 1) for (x, y) in (v0, v1, v2)]
    # sort by x ascending (stable compare-swaps as in disasm: swap only on strict >)
    A, B, C = verts
    if A[0] > B[0]: A, B = B, A
    if B[0] > C[0]: B, C = C, B
    if A[0] > B[0]: A, B = B, A
    Ax, Ay = A; Bx, By = B; Cx, Cy = C
    table = {}
    if Ax != Bx and Bx != Cx:
        dda_init(Ay, Ax, Cy, Cx, table)
        dda_union(Ay, Ax, By, Bx, table)
        dda_union(By, Bx, Cy, Cx, table)
    elif Ax == Bx and Bx != Cx:
        dda_init(Ay, Ax, Cy, Cx, table)
        dda_union(By, Bx, Cy, Cx, table)
    elif Bx == Cx and Ax != Bx:
        dda_init(Ay, Ax, By, Bx, table)
        dda_union(Ay, Ax, Cy, Cx, table)
    else:
        # all x equal: vertical line, route elsewhere
        for x in (Ax,):
            ys = sorted([Ay, By, Cy]); table[x] = (ys[0], ys[2])
    # produce lit screen pixels: column x, y' in [lo,hi] -> screen_y = H - y' - 1
    lit = set()
    for x, (lo, hi) in table.items():
        if lo > hi: lo, hi = hi, lo
        for yp in range(lo, hi + 1):
            lit.add((x, H - yp - 1))
    return lit

def parse_mask(tsv_line, box_w, box_h, ox, oy):
    vals = tsv_line.split('\t')[1].split()
    lit = set()
    idx = 0
    for dy in range(box_h):
        for dx in range(box_w):
            if vals[idx] != '0':
                lit.add((ox + dx, oy + dy))
            idx += 1
    return lit

# masks: name -> (verts, box(w,h,ox,oy))
CASES = {
    'lineA': (((0,0),(1,9),(0,9)), (2,10,0,0)),
    'lineB': (((0,0),(9,9),(0,9)), (10,10,0,0)),
    'lineC': (((0,0),(20,3),(0,3)), (21,4,0,0)),
    'lineD': (((0,0),(3,20),(0,20)), (4,21,0,0)),
    'lineE': (((0,0),(9,4),(0,4)), (10,5,0,0)),
    'skewA': (((0,0),(20,0),(20,6)), (21,7,0,0)),
    'skewB': (((0,0),(13,0),(0,7)), (14,8,0,0)),
    'skewC': (((10,10),(40,13),(10,20)), (31,11,10,10)),
    'skewD': (((3,0),(9,20),(0,12)), (10,21,0,0)),
    'skewE': (((0,0),(8,3),(3,9)), (9,10,0,0)),
}

import sys
tsvs = {}
for f in ('out/gtri_fill_line.tsv', 'out/gtri_fill_skew.tsv'):
    for line in open(f):
        if line.strip() and not line.startswith('#'):
            tsvs[line.split('\t')[0]] = line.rstrip('\n')

for H in [512, 256, 240, 192, 511, 255]:
    total = 0
    detail = []
    for name, (verts, box) in CASES.items():
        w, h, ox, oy = box
        truth = parse_mask(tsvs[name], w, h, ox, oy)
        pred = gtri_fill(*verts, H)
        # restrict pred to box
        predbox = {(x, y) for (x, y) in pred if ox <= x < ox+w and oy <= y < oy+h}
        err = len(truth ^ predbox)
        total += err
        detail.append(f"{name}:{err}")
    print(f"H={H}: total_err={total}  " + " ".join(detail))
