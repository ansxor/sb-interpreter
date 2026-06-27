# GTRI filled-triangle rasterizer — reverse-engineering notes (bd:sb-interpreter-j4l)

Status: **algorithm decoded from disassembly; ~97% (≈680/700 px over the 5 skew masks)
reproduced by both a float-band model and an integer-Bresenham model.** The remaining ~3%
is an exact sub-pixel seed/tie-break I have not yet pinned. NO interpreter code shipped this
run (sb-core `gtri` stays at the barycentric baseline). Working models live in
`gtri_fill_model.py` (band_model: 21px wrong; dda_union: 44px wrong, over the 5 masks).

## CORRECTION to the prior bead lead
The earlier bead NOTES pointed NEXT STEP at `FUN_001556d8` / `FUN_001557f4` ("the float
helpers right after the GTRI handler", vmla.f32/vcvt). **Those are NOT the triangle fill** —
they are GPUTCHR / text-pen helpers (they iterate a UTF-16 string, skip `0x20` spaces, read
glyph metrics, and accumulate the pen-x at ctx `+0x3a8`/`+0x3ac`). They sit adjacent to the
GTRI handler in the binary but are unrelated. The real chain is below.

## Real call chain (all addresses in cia_3.6.0.lst)
- `GTRI` handler `@0x15549c` (`FUN_00154ed0` dispatch) — evaluates 6 coord args (argc 6) or
  7 (argc 7 w/ color) into the handler frame `[sp+0x10..0x24]` = x1,y1,x2,y2,x3,y3, color at
  `[sp+0xc]`, then `bl 0x00194120`.
- `FUN_00194120` — the fill. Does:
  1. **Y-FLIP**: every vertex Y is replaced by `Y' = ctx[8] - Y - 1` (ctx[8] = surface
     height). X unchanged. So scan-conversion happens in physical (flipped) row space; the
     asymmetric Bresenham rounding lands accordingly. (This is why a naive top→bottom fill is
     off by ~1px on sloped edges in one direction and the flipped one is off the other way —
     the truth sits between, i.e. the flip matters.)
  2. **Sort the 3 vertices by X ascending** (3 compare-swaps at `0x194164`/`0x194184`/
     `0x1941a8`, swapping both the X slot `[sp+0x4/0x8/0xc]` and the Y' slot
     `[sp+0x14/0x18/0x1c]`).
  3. Degenerate / fully-clipped guards (`0x1941d0`..`0x194294`); collinear routes to the line
     helpers (`0x155f8c`+`0x15cab0` via the `0x1942ec` branch) — already handled by sb-core's
     existing degenerate→line_dev path.
  4. **Build a per-scanline span table** at `0x1B1E33C` (array of `{i16 lo, i16 hi}` indexed
     by Y'): one edge **initialises** via `FUN_00155f8c`, the other two **union** (min lo /
     max hi) via `FUN_0015cab0`. The init edge is the one spanning the full Y' range (top↔
     bottom vertex) so every scanline is written.
  5. **Fill loop** `@0x194374`: per Y', read `{lo,hi}`, swap so lo≤hi, clamp to the write
     clip `[ctx+0x10, ctx+0x14]`, and write a horizontal run via the tile-swizzled span
     writer `FUN_001944e8` (LUTs `0x2C9AE0`/`0x2C9B60`, 8×8 tile addressing — same swizzle as
     the GLINE DDA `FUN_001e6700`).

## The edge DDA (`FUN_00155f8c` init, `FUN_0015cab0` union) — exact integer Bresenham
Both take `(x0,y0,x1,y1)` and require `y0<=y1` (else return without writing). Setup:
```
adx=|x1-x0| ; ady=|y1-y0| ; r5=2*adx ; r4=2*ady
err = (adx>ady) ? -adx : ady        ; <-- seed (see OPEN QUESTION)
```
- **vertical** (x0==x1): write `[x0,x0]` for every y in [y0,y1].
- **x increasing** (x0<x1), per scanline y=y0..y1:
  ```
  err += 2*adx ; old = x
  while (err >= 2*ady && x <= x1) { x += 1 ; err -= 2*ady }   ; do-while: steps >=1 if entered
  new = x ; run = [old, (old==new ? new : new-1)]
  ```
- **x decreasing** (x0>x1): symmetric, `run = [(old==new ? new : new+1), old]`.

`FUN_00155f8c` stores the run; `FUN_0015cab0` merges `lo=min(existing,run_lo)`,
`hi=max(existing,run_hi)`. (Verified store sites: inc `@0x15604c`/`@0x15cb64`, dec
`@0x1560a8`/`@0x15cbb8`.)

So the fill span per scanline = **union over the 3 edges of each edge's Bresenham run**.

## What reproduces the masks
- A **float center-band model** — left=`ceil(min_x)`, right=`floor(max_x)` where min/max are
  the edge x's over the band `[y-0.5, y+0.5)` (half-open at the bottom for *non-vertical*
  edges; vertical edges closed) — reproduces **skewA, skewB exactly** and skewC to 1px, but
  under-fills steep edges (skewD/E) because integer Bresenham "fattens" a steep edge by a
  pixel near the apex that the geometric band misses. → see `scratchpad gtri8.py` logic.
- The **integer-DDA union model** above reproduces the steep cases' fattening but is off by a
  consistent ~1px phase on shallow edges in whichever Y direction; no fixed seed-offset or
  run-endpoint convention closes it (brute force over flip × seed deltas × 16 endpoint
  conventions bottoms out at ~19–23 wrong px).

## OPEN QUESTION (for next run)
The exact **seed `err0`** (sub-pixel phase) of `FUN_00155f8c`. The disasm literally sets
`err0 = (adx>ady)?-adx:ady`, but a verbatim port is ~1px out of phase vs the harvested masks
in both flip orientations (truth is exactly between). Candidates to chase:
1. Re-check whether `FUN_00194120` pre-biases the coords before the DDA (an X or Y ±0.5 / ±1
   nudge I may have mis-tracked through the X-sort + Y-flip register juggling at
   `0x194140`..`0x1941cc`). The `sub …,#1` at `0x194150/54/58` (the `Y'-1`) is the prime
   suspect — confirm it's only the flip and not an extra bias.
2. Confirm which physical endpoint each of the 3 DDA calls receives (decode the register/stack
   loads at `0x1944a0`..`0x194334`) — I assumed top↔bottom-by-Y' for init and the two shorts
   for union; the X-sort may reassign which concrete edge is init vs union.
3. Harvest 2–3 more **single-sloped-line** masks (thin triangles, e.g. `GTRI 0,0,1,9,0,9` and
   a near-45°) to pin the seed phase directly, then port `FUN_00155f8c` verbatim.

Once the seed is pinned: port `FUN_00155f8c`/`FUN_0015cab0` verbatim into `sb-render` `gtri`
(replacing the barycentric fill), add a GTRI scene golden from `gtri_fill_skew.tsv`, and raise
to hw_verified. Ground truth already harvested: `out/gtri_fill_skew.tsv` (skewA–E) +
`gtri_fill_skew_cases.txt`.

---

## RUN 3 (2026-06-27): single-edge masks harvested → integer-DDA-verbatim DISPROVEN

Harvested 5 thin right-triangle masks `GTRI 0,0,W,H,0,H` (vertical left edge x=0, hypotenuse
(0,0)->(W,H), so the right boundary per scanline directly reveals the edge rule for slope W/H).
Ground truth: `out/gtri_fill_line.tsv`, cases `gtri_fill_line_cases.txt`, decode/brute scripts
`gtri_seed_probe.py` + `gtri_seed_brute.py`. Slopes 1/9, 9/9, 20/3, 3/20, 9/4.

Right-boundary truth per scanline:
```
lineA 1/9 : [0,0,0,0,1,1,1,1,1,1]      (step at y=4, x(4)=0.444)
lineB 9/9 : [0,1,2,3,4,5,6,7,8,9]      (= y exactly)
lineC 20/3: [3,9,16,20]                (shallow: boundary = far end of the row's x-run)
lineD 3/20: [0,0,0,1,1,1,1,1,1,1,2,2,2,2,2,2,3,3,3,3,3]
lineE 9/4 : [1,3,5,7,9]
```

### Disasm seed + endpoint CONFIRMED (and it is the WRONG model for the fill)
Read `FUN_00155f8c` (init) and `FUN_0015cab0` (union) bodies. BOTH use identical Bresenham:
- seed `err0 = (adx>ady) ? -adx : ady` — `cmp r4,r6; rsbgt r12,r4,#0; cpyle r12,r6` @0x155FA0.
- per-scanline: `err += 2adx`; step `do{x+=1; err-=2ady}while(x<=x1 && 2ady<=err)`.
- store (x-increasing): `lo = old`, `hi = (old==new ? old : new-1)` — `cmp r8,r0; cpyeq r8,r0;
  subne r8,r0,#1` @0x15cb84 (union) / @0x15605c (init). x-decreasing mirrors (lo=new+1, hi=old).
- table indexed by the raw (flipped) y, clamp `cmp r1,#0x200` (512) — GRP page is 512 tall.

Plugging this VERBATIM (seed err0, hi=new-1, the 3-edge min/max union, Y-flip by 512 OR 240 OR
none) into `gtri_seed_brute.py` bottoms out at **44–55 px wrong** over the 10 masks — and on the
steep lines it steps exactly **one scanline LATE in BOTH flip orientations** (lineA predicts the
x=1 boundary starting at y=5; truth starts at y=4). A late step in *both* flips means no
flip/seed choice rescues it — the geometry the HW fills is simply not this integer DDA.

### Conclusion: the general GTRI fill is FLOAT, not the integer Bresenham union
- float center-band model = **21 px** wrong (exact on skewA/B and all axis-aligned/shallow edges,
  via the half-open `[y-0.5,y+0.5)` x-sweep — see lineC fits exactly); integer DDA = 44+ px.
- steep edges want `right ≈ floor(x(y) + ~0.6)` (a center-sample with a constant bias), which a
  half-open geometric band cannot express simultaneously with the shallow half-open boundary —
  so the remaining ~21 px is a single sub-pixel fill-convention (likely pixel-center / top-left
  edge-function eval at `(x+0.5, y+0.5)`), NOT a Bresenham seed.
- This RETIRES the bead's standing NEXT-STEP ("port FUN_00155f8c verbatim"): those two helpers
  (0x155f8c/0x15cab0) are the LINE/collinear-edge path, not the triangle interior fill.

### NEXT (for the next run)
1. Disassemble the float math actually inside `FUN_00194120`'s span build (the vmul/vcvt/vadd
   around the table writes @0x1B1E33C) — find the real per-scanline edge-x float formula and its
   round-to-int (vcvt mode). That, not the integer DDA, is the fill.
2. With that formula, close the remaining 21 px (steep-edge phasing) to 0, then port into
   `sb-render` gtri (replacing barycentric) + add a GTRI scene golden from the line+skew masks.

---

## RUN 4 (2026-06-27): SOLVED — GTRI fill is a TRANSPOSED integer Bresenham. Shipped.

RUN 3's NEXT-STEP ("disasm the FLOAT math inside FUN_00194120's span build") was a dead end:
there is NO float math in `FUN_00194120` (@0x194120..0x1944e8) — the whole span build is the
integer helpers `FUN_00155f8c` (init) + `FUN_0015cab0` (union). The error in RUNs 2-3 was the
**axis**: those runs simulated the DDA per-SCANLINE (table indexed by Y, horizontal spans).

### The fix: the DDA is run TRANSPOSED (indexed by X, vertical spans)
Reading the edge-call register loads in `FUN_00194120` (general path @0x1944a0-0x1944d8):
the calls pass `r0=Ay', r1=Ax, r2=Cy', r3=Cx` — i.e. **(value=Y', index=X)**. Confirmed
against the helper bodies: `FUN_00155f8c`'s loop variable is `r1` (the table index) and it
runs from `r1`→`r3`; the vertical branch (`r0==r2`) writes a constant value across that index
range. So the span table @0x1B1E33C is **indexed by X (columns)**, each entry holds a
`{lo,hi}` **Y** range, and the fill loop @0x194374 writes **VERTICAL** runs (the swizzle uses
`r1`=X as the column and `r0`=Y as the row start). The DDA ALWAYS drives along X regardless of
slope (the seed `err0=(dV>dI)?-dV:dI` handles steep edges via the multi-step inner loop).

Structure (after Y-flip `y'=ctx[8]-y-1` and X-ascending sort A≤B≤C):
- **init** edge A→C (min-x to max-x) seeds every column — `FUN_00155f8c`, OVERWRITE.
- **union** edges A→B and B→C merge their sub-ranges — `FUN_0015cab0`, min-lo / max-hi.
- two-equal-x dispatch (@0x19427c / @0x1942ec) drops the vertical edge: init A→C + union B→C
  (when Ax==Bx) or init A→B + union A→C (when Bx==Cx).
- fill: per column x in [Ax..Cx], `[lo,hi]` → vertical run.

The Y-flip is **flip-invariant** for the final pixel set (the asymmetric `new-1`/`new+1`
rounding reverses with the axis and un-reverses on read-back) — verified H∈{512,256,240,192,
none} all give 0 error — so we rasterize directly in screen Y.

### Verified EXACT against all ground truth
A faithful Python port (`harness/harvest/gtri_transposed.py`) of init+union+driver reproduces ALL
10 harvested masks (lineA-E + skewA-E) with **0 px error**, and the spec's committed 45° mask
(`GTRI 100,100,104,100,100,104` = dx+dy≤4). A closed-form per-column span (O(1), needed so
extreme i32 coords don't iterate 2^31 columns) matches the iterative device loop over **1e6
random edges** with 0 mismatches.

### Shipped
- `crates/sb-render/src/raster.rs`: replaced the barycentric `gtri` fill with the transposed
  Bresenham (`gtri_edge_col` closed form + `floor_div`). Degenerate→line_dev path unchanged.
- New test `gtri_fill_matches_device_masks` replays all 10 GSPOIT masks (hw_verified).
- `spec/instructions/gtri.yaml`: corrected the "sorted by y then x" prose (it sorts by X
  only) and documented the transposed-Bresenham fill in the disassembled ref.
- bd:sb-interpreter-j4l CLOSED. The model is now hw_verified at the pixel level.
