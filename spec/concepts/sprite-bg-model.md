---
title: Sprite & BG model
slug: sprite-bg-model
area: sprites-bg
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/sprites.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/bg-backgrounds.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/constants.md (#SP*/#BG*/#CHK*)" }
  - { type: hw_verified, ref: "spec/reference/constants.yaml — #SP*/#BG*/#CHK* attribute & channel bits (all hw_verified, S-T14c)" }
  - { type: disassembled, ref: "sprite handlers: SPANIM @0x141f.., SPCOL @0x13f9.., SPHITSP @0x1438.., SPLINK @0x1429.., SPVAR @0x141d.., SPCHK @0x13ed.., SPCHR @0x13ee.., SPHOME @0x1427.. — see the cited spec/instructions/*.yaml (handler bodies read there)" }
  - { type: disassembled, ref: "BG handlers: BGSCREEN @0x16696.., BGPUT @0x1646.., BGOFS @0x1642.., BGANIM @0x164c.., BGCHK @0x163f.., BGVAR @0x164a.., BGROT @0x1648.., BGSCALE @0x1666.., state ptr 0x00315D60 / layer-record base 0x019A2F50, stride 0xe3*8" }
related:
  - SPSET
  - SPCLR
  - SPSHOW
  - SPHIDE
  - SPOFS
  - SPSCALE
  - SPROT
  - SPHOME
  - SPCHR
  - SPPAGE
  - SPANIM
  - SPSTART
  - SPSTOP
  - SPFUNC
  - SPLINK
  - SPUNLINK
  - SPVAR
  - SPCHK
  - SPDEF
  - SPUSED
  - SPCOL
  - SPCOLVEC
  - SPHITSP
  - SPHITRC
  - SPHITINFO
  - SPCLIP
  - SPCOLOR
  - BGSCREEN
  - BGPAGE
  - BGPUT
  - BGGET
  - BGFILL
  - BGCLR
  - BGOFS
  - BGROT
  - BGSCALE
  - BGHOME
  - BGCLIP
  - BGCOLOR
  - BGSHOW
  - BGHIDE
  - BGANIM
  - BGSTART
  - BGSTOP
  - BGVAR
  - BGFUNC
  - BGCHK
  - BGCOORD
  - BGCOPY
  - BGLOAD
  - BGSAVE
confidence: documented
---

# Sprite & BG model

The contract for M3 (sprite + BG subsystems) and how they composite into the M2
framebuffer. Two object families — **sprites** (free-moving images, dot coordinates) and
**BG** (tiled backgrounds, tile coordinates) — share the same depth model, the same
keyframe-animation engine, and the same per-object variable mechanism, but differ in their
storage and addressing. Numbers below are cited to the docs and to the disassembled
instruction specs under `spec/instructions/` (which carry the handler-body reads); see
[[screen-and-color-model]] for where these layers sit in the compositor.

## Where they sit in the layer stack

Back → front (per [[screen-and-color-model]]): backdrop → GRP pages → **BG** → **sprites**
→ console. But this back-to-front default is only the *base* ordering — both sprites and BG
carry an explicit **Z (depth) coordinate** that overrides it, so a sprite can be pushed
behind a BG layer or a BG layer pulled in front of a sprite. Z range is shared and identical
to GRP: **rear 1024 … screen surface 0 … front −256**; smaller Z draws in front. Z also
drives stereoscopic depth in 3D mode. (Set via `SPOFS …,Z` and `BGOFS …,Z`.)

---

# Sprites

## Sprite table

- **512 sprites**, management numbers **0–511** ([[SPSET]] docs; the shared sprite-index
  getter `FUN_001eece0` rejects anything outside `0..511` with **errnum 10 Out of range**).
- Each sprite is one fixed-size record in a global table (sprite-record base differs per
  handler literal, e.g. `0x019A5FB0`; index → slot via the `*0x48`-style stride
  `rsb/add/add` sequence seen in every sprite handler). Records are addressed as
  `(mgmt + page_base) * stride`, where `page_base` comes from the current **sprite display
  page** ([[SPPAGE]]; the page base is field `+0x44` of the sprite state — collisions and
  lookups are scoped to the active page).
- **Active bit**: slot flags live at record offset **`+0x4`**; **bit 31 (`0x80000000`)** is
  the "this sprite is SPSET" flag. Most instructions (`SPOFS`, `SPCHR`, `SPHOME`, `SPCOL`,
  `SPCHK`, …) test it and raise **errnum 4** ("used before SPSET") when clear. Exceptions
  that deliberately do **not** require it: [[SPVAR]] (variables exist pre-SPSET) and the
  `SPHIT*` family (see Collision — they return "no hit" rather than erroring).
- [[SPUSED]] reports whether a management number is currently in use; [[SPCLR]] releases a
  sprite (clears the active bit) and frees its memory. `SPCLR` with no argument clears all.

### Slot record offsets (disassembled — for the implementation)

| Offset | Field | Source |
|---|---|---|
| `+0x4` | flags word: bit 31 active, bit `0x2000000` anim-stopped, channel-status in high bits | [[SPCHK]], [[SPANIM]] |
| `+0x18` | home point HX,HY (f32) | [[SPHOME]] |
| `+0x30 / +0x34` | current X,Y position (f32) — base for relative `SPANIM "XY"` | [[SPANIM]] |
| `+0x40 / +0x42` | image U,V (i16) | [[SPCHR]] |
| `+0x58 …` | the 8 user variables `SPVAR 0..7` (consecutive **doubles**, `+0x58 + n*8`) | [[SPVAR]] |
| `+0xa4` | parent pointer for [[SPLINK]] (0 = unlinked) | [[SPLINK]] |

## Image source & definition templates

- Sprite art is preloaded in **GRP4** (docs). The displayed image is a rectangle of the
  sprite sheet: **U,V** = top-left, **W,H** = size (default **16×16**). `U+W ≤ 512` and
  `V+H ≤ 512` (else errnum 10).
- [[SPDEF]] holds the **definition-template table**: numbers **0–4095** (`0..0x1000`,
  clamped to 4096 templates), each packing `U,V,W,H` + home `HX,HY` + default attribute `A`
  as 7 fields. The table is **shared by both screens** and is seeded from `spdef.csv`
  (`SPDEF` with no args resets it). `SPSET mgmt, defn` and `SPCHR mgmt, defn` create/replace
  from a template; `SPSET mgmt, U,V[,W,H][,attr]` specifies the image directly.

## Transforms

| Instr | Effect | Notes |
|---|---|---|
| [[SPOFS]] | position X,Y + optional **Z** depth | dot units; Z range 1024…0…−256 |
| [[SPSCALE]] | magnification X,Y | `1.0`=100%; X,Y independent; no documented clamp |
| [[SPROT]] | rotation angle, **clockwise degrees** | negative / >360 allowed (sprites do not pre-normalize the stored angle the way BG does) |
| [[SPHOME]] | base point HX,HY | relative to top-left; default `(0,0)`; **floats**, negatives & fractions allowed (e.g. `-16`, `127.5`) — the pivot for scale/rotate and the origin for `SPCOL` |
| [[SPCHR]] | change image / size / attribute after creation | empty middle args keep the current value; template form sets bit `0x40` |
| [[SPPAGE]] | which sprite display page this sprite belongs to | adds `page_base` to the slot index |

Transform application order (pivoting around SPHOME): translate to home → scale → rotate →
place at SPOFS. (Exact rounding/order vs the renderer is an M3 golden-diff item.)

## Attribute bitfield (shared by SPSET / SPCHR `attr`)

All `hw_verified` in `spec/reference/constants.yaml`:

| Bit | Mask | Constant | Meaning |
|---|---|---|---|
| b0 | `&H01` | `#SPSHOW` | display ON (default for a new sprite) |
| b1–b2 | `&H00/02/04/06` | `#SPROT0/90/180/270` | 90° step rotation |
| b3 | `&H08` | `#SPREVH` | horizontal flip |
| b4 | `&H10` | `#SPREVV` | vertical flip |
| b5 | `&H20` | `#SPADD` | additive blending |

Valid attribute range is `0..0x3f` (SPDEF array form validates this). The 90° rotation bits
compose with the free-angle [[SPROT]] rotation.

## Animation (the keyframe engine)

[[SPANIM]] drives per-**channel** keyframe animation. A *target* selects the channel and may
be given as a number **or** a string name (parsed by the shared name parser `FUN_001836cc`):

| Target name | Channel bit | Animates | Items/keyframe |
|---|---|---|---|
| `"XY"` | `#CHKXY` (1) | position | 2 (x,y) |
| `"Z"` | `#CHKZ` (2) | depth | 1 |
| `"UV"` | `#CHKUV` (4) | image U,V | 2 |
| `"I"` | `#CHKI` (8) | definition number | 1 |
| `"R"` | `#CHKR` (16) | rotation | 1 |
| `"S"` | `#CHKS` (32) | scale | 2 (sx,sy) |
| `"C"` | `#CHKC` (64) | color | 1 |
| `"V"` | `#CHKV` (128) | variable 7 (`SPVAR …,7`) | 1 |

- A trailing **`+`** on the target name (e.g. `"XY+"`, the bit-3 "relative" flag) makes the
  keyframe values **relative** to the sprite's current value at start, instead of absolute.
- **Up to 32 keyframes** per target (documented cap). Each keyframe has a **time**:
  **positive = hold** that many frames; **negative = linear interpolation** over `|time|`
  frames toward the keyframe value.
- Animation **starts on the frame *following*** the `SPANIM` call (documented) and runs
  **once**, unless a **loop count** is given — **loop 0 = endless**.
- [[SPSTART]] / [[SPSTOP]] resume / pause an animation by toggling the slot's stop bit
  `0x2000000`.
- [[SPCHK]]`(mgmt)` returns which channels are currently animating, as the OR of the `#CHK*`
  bits above (`status = (flags >> 17) & 0xFF`). **0 = all stopped.** If the stop bit is set,
  `SPCHK` returns 0.
- [[SPFUNC]] registers a per-sprite callback (with `CALLIDX` identifying the sprite inside
  it); fired by the animation/`CALL SPRITE` machinery.

> ⚠ The `#CHK*` constants are named "collision check flags" in the docs/constant table but
> in `SPCHK`/`BGCHK` they are **animation-channel** flags. The collision system uses a
> separate 32-bit **mask** (below), not these bits.

## Linking ([[SPLINK]] / [[SPUNLINK]])

- `SPLINK child, parent` makes `child` inherit the **parent's position** (X,Y only — **not**
  rotation or scale, documented). Used to build multi-jointed characters.
- **Ordering rule (enforced):** the parent number must be **strictly lower** than the child
  (`child > parent`, else errnum 4). Both must be SPSET.
- `SPLINK(child)` as a function returns the child's parent number, or **−1** if unlinked
  (read from slot `+0xa4`). [[SPUNLINK]] removes the link.

## Per-sprite variables ([[SPVAR]])

- **8 variables per sprite**, numbers **0–7**, stored as doubles at slot `+0x58 + n*8`.
- Usable as setter `SPVAR m,n,v`, function `v=SPVAR(m,n)`, or `SPVAR m,n OUT v`.
- **Does not require SPSET** — variables exist (default 0) before a sprite is created.
- **Variable 7** is special: writing it also clears slot flag `0x1000000`; it is the target
  of the `SPANIM "V"` channel.
- Out-of-range variable number (outside 0–7) behaviour is **oracle-pending** — sprite `SPVAR`
  has no visible explicit guard at the store site (unlike BG `BGVAR`, which guards 0–7).

## Collision

The collision system is **swept** (moving-quadrangle), float-precision, and gated by a
per-sprite bitmask.

- [[SPCOL]] **enables** collision for a sprite and configures it:
  - **Detection rectangle** — defaults to the sprite's full definition size `(0,0,W,H)`,
    measured **relative to SPHOME** (the home point is the rect origin). The explicit form
    overrides with `start_x,start_y` (−32768..32767) and `width,height` (0..65535).
  - **Mask** — a **32-bit** value (default `&HFFFFFFFF` = all bits). Two sprites collide only
    when `(maskA AND maskB) != 0`.
  - **Scale-sync** — a boolean; TRUE keeps the detection rect synced to `SPSCALE`, but only
    for `SPSCALE`s issued **after** this `SPCOL`.
  - `SPCOL` also has OUT/getter forms (read back scale/mask/range).
- [[SPCOLVEC]] sets the sprite's collision **velocity** (movement vector, floats). With no
  X,Y it auto-derives the vector from the previous frame's `SPANIM "XY"` interpolation; the
  vector is what the swept test uses and what `SPHITINFO` reports as VX/VY.
- [[SPHITSP]] tests sprite-vs-sprite. Forms by argcount: `SPHITSP(m)` = m vs all (returns the
  first colliding number or **−1**); `SPHITSP(m1,m2)` = pair (returns TRUE/FALSE);
  `SPHITSP(m1,start,end)` = m1 vs a range. Page-scoped (page_base added).
- [[SPHITRC]] tests a sprite/quadrangle against an arbitrary rectangle (vs all / vs one / vs
  range, with optional mask + movement). Float math (VFP), swept.
- [[SPHITINFO]] returns the last hit's **time + coordinates + velocities** (forms with 1, 3,
  5, or 9 OUT vars; `coord = position_at_detection + velocity * time`, time clamped to
  `0..1` of the frame). The 3-var form is undocumented but accepted (oracle-pending).
- **Important divergence from the docs:** the docs say the `SPHIT*` functions error if used
  before SPSET; the disassembly + oracle show they **do not** — they return "no collision"
  (`-1` / `FALSE(0)` / default record) for an in-range but un-SPSET sprite. An error
  (**errnum 10**) is raised only when a management number is itself outside `0..511`.
  (hw_verified 2026-06-22: `SPCLR:SPHITSP(5)`→−1, `SPCLR:SPHITRC(1,0,0,16,16)`→0,
  `SPCLR:SPHITINFO OUT TM`→0.)
- [[SPCLIP]] sets a per-sprite clipping rectangle; [[SPCOLOR]] a display-color multiply.

---

# BG (Backgrounds)

## Layers & screens

- **4 layers per screen**, numbers **0–3** (the shared BG-layer getter `FUN_001e2504`
  rejects `< 0` or `≥ 4`, where `4` is the live layer count at state `[0x00315D60]+0x60`,
  with **errnum 10**). Unlike sprites, **BG layers 0–3 always exist** — there is no
  active/SPSET-style guard, so `BGANIM`/`BGCHK`/`BGVAR` never raise a "used before setup"
  errnum 4.
- BG coordinates are in **tile units** (16×16-dot tiles), *not* dots — contrast sprites.
- BG tile art is preloaded in **GRP5** (docs).
- Default map size is **25 × 15 tiles** (fills the top screen at 16×16). [[BGSCREEN]] resizes
  a layer's map: `WIDTH,HEIGHT ≥ 1`, and **`WIDTH * HEIGHT ≤ 16383`** (`0x3fff`, errnum 10
  otherwise). An optional 4th arg sets **tile size** — `8`, `16` (default), or `32` px (any
  other value → errnum 4).
- [[BGPAGE]] selects the BG drawing page; [[BGSHOW]] / [[BGHIDE]] toggle layer visibility;
  [[BGCLR]] clears a layer (or all).
- Per-layer record: base `[0x00315D60]+0x5c` (literal `0x019A2F50`), **stride `0xe3 * 8`
  bytes**; the flags word is the record's first word.

## Tilemap cell format ([[BGPUT]] / [[BGGET]] / [[BGFILL]])

`BGPUT layer, x, y, data` writes one cell; `x ∈ 0..width-1`, `y ∈ 0..height-1` (else errnum
10). The **screen data** is a 16-bit value (a number, or a 4-hex-digit string `"0000".."FFFF"`):

| Bits | Field |
|---|---|
| b0–b11 | **character number 0–4095** (display repeats with a cycle of **1024**) |
| b12–b13 | rotation: `#BGROT0/90/180/270` = `&H0000/1000/2000/3000` |
| b14 | `#BGREVH` (`&H4000`) horizontal flip |
| b15 | `#BGREVV` (`&H8000`) vertical flip |

Character number **0 displays nothing** (empty/transparent cell). [[BGFILL]] fills a
rectangle with one cell value; [[BGGET]] reads a cell back; [[BGCOPY]] copies a region.

## Transforms

| Instr | Effect | Notes |
|---|---|---|
| [[BGOFS]] | scroll X,Y (dots) + optional **Z** depth | per-layer; SET (3/4 args) or OUT (read X,Y[,Z]); positive X/Y move left/up (docs) |
| [[BGROT]] | rotation angle | **normalized mod 360** before storing (−90→270, 450→90); pivots around BGHOME |
| [[BGSCALE]] | scale X,Y (floats) | **no clamp** to the documented 0.5–2.0 — any value accepted; downscaling past 3600 visible cells distorts (render limit, not an error) |
| [[BGHOME]] | layer origin (rotation/scale pivot) | |
| [[BGCLIP]] | per-layer clip rectangle | |
| [[BGCOLOR]] | layer color multiply | |
| [[BGCOORD]] | coordinate conversion | OUT DX,DY required; **mode 0** BG→display, **mode 1** display→BG (tile units), **mode 2** display→BG (pixel units); applies the layer's current scroll/rot/scale/home so it round-trips |

## Animation

[[BGANIM]] mirrors `SPANIM` (same name parser, same keyframe model, same 32-keyframe cap,
same start-next-frame + loop-0-endless rules), per BG layer. BG has **fewer channels** —
it lacks the sprite-only `UV`(`#CHKUV`,4) and definition `I`(`#CHKI`,8) channels:

| Target | Channel bit | Animates |
|---|---|---|
| `"XY"` | `#CHKXY` (1) | scroll offset |
| `"Z"` | `#CHKZ` (2) | depth |
| `"R"` | `#CHKR` (16) | rotation |
| `"S"` | `#CHKS` (32) | scale |
| `"C"` | `#CHKC` (64) | color |
| `"V"` | `#CHKV` (128) | variable 7 |

- [[BGSTART]] / [[BGSTOP]] toggle the layer-record **stop bit `0x40`** (not the sprite's
  `0x2000000`).
- [[BGCHK]]`(layer)` returns the running-channel bits. **Unlike `SPCHK`, there is NO
  `>>17 & 0xFF` shift** — BG's running bits live in the **low byte** of the layer flags word;
  the stop bit `0x40` forces the result to 0. (Which exact bit is set while a given channel
  runs is **oracle-pending** — see HARVEST_QUEUE.md.)
- [[BGVAR]]: 8 variables (doubles) per layer, **explicitly range-guarded 0–7** (errnum 10
  outside) — stricter than sprite `SPVAR`. Writing variable 7 clears layer flag bit `0x20`
  (the `BGANIM "V"` marker). No setup required (default 0).
- [[BGFUNC]] registers a per-layer callback (`CALLIDX`).

## Load / save

[[BGLOAD]] / [[BGSAVE]] move a layer's tilemap to/from an array or resource; round-trip
fidelity is an M3 acceptance item.

---

## Shared conventions (both families)

- **Page-base addressing**: a sprite's/layer's record index has the current display-page base
  added before the stride multiply, so sprite/BG state is per-page.
- **Target-name parser** `FUN_001836cc`: the same routine resolves `"XY"/"Z"/"UV"/"I"/"R"/
  "S"/"C"/"V"` (+ trailing `+`) for both `SPANIM` and `BGANIM`; a non-string/non-numeric
  target → **errnum 8 Type mismatch**, a negative resolved target → **errnum 4**.
- **Setter vs getter** is selected by the *return (OUT) count*, not by syntax: `SPVAR`/`BGVAR`/
  `SPCHK`/`BGCHK` treat `v=F(...)` and `F(...) OUT v` identically (the handler can't tell them
  apart). OUT slots may be **skipped with empty commas** (e.g. `BGOFS L OUT ,,Z`,
  `SPCOL id OUT ,mask`).
- **Common errnums**: out-of-range mgmt/layer → **10**; bad arg/return shape, used-before-
  SPSET, bad target → **4**; wrong operand type → **8**; SPDEF array count not a multiple of
  7 → **31**.

## Open questions → oracle (queued in HARVEST_QUEUE.md)

- Exact per-channel **mid-animation bit values** that `SPCHK`/`BGCHK` report while a given
  channel runs (the `#CHK*` mapping is documented; the live bit pattern is unconfirmed).
- Sprite `SPVAR` **out-of-range variable number** behaviour (no visible guard at the store).
- Transform/compositing exactness: **draw/pivot order** (scale vs rotate vs scroll origin),
  rounding, and **Z tie-breaking** between sprites, BG layers, GRP, and console — pin via the
  M3 golden PNG diffs (O-T6) against the oracle framebuffer.
- `SPHITINFO` **3-variable OUT form** (accepted by the handler, undocumented) — confirm.
