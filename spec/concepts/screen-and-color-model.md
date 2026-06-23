---
title: Screen & color model
slug: screen-and-color-model
area: graphics
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/screen-layout.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/computer-colors-rgb.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/constants.md" }
  - { type: disassembled, ref: "cia_3.6.0.lst pixel-read helper FUN_00191dfc @0x191e40..@0x191e90: ldrh r0,[r0] (16-bit device pixel) then channel unpack with preloaded masks mov r2,#0xf8 / r3,#0xf800 / r1,#0xf80000 and shifts `and r2,r2,r0,lsl#0x2` (blue from pixel[5:1]) / `and r3,r3,r0,lsl#0x5` (green from pixel[10:6]) / `and r1,r1,r0,lsl#0x8` (red from pixel[15:11]); alpha is `tst r0,#0x1` (bit 0) -> `orrne r0,r0,#0xff000000`. The masks 0xf8/0xf800/0xf80000 keep only the high 5 bits of each output byte (low 3 bits forced 0), so 5->8 expansion is LEFT-SHIFT ONLY, NOT (v<<3)|(v>>2). Off-page read returns mov r0,#0x0 @0x191e8c." }
  - { type: hw_verified, ref: "spec/reference/constants.yaml (S-T14c, all 79 hw_verified): #WHITE=&HFFF8F8F8, #RED=&HFFF80000, #BLUE=&HFF0000F8, #CYAN=&HFF00F8F8 — every channel maxes at 0xF8=248=31<<3 (NOT 0xFF), proving 5-bit<<3 expansion." }
  - { type: hw_verified, ref: "sb-oracle batch 2026-06-22 (s_c2 round-trip): GPSET x,y,RGB(255,0,0) then GSPOIT(x,y) -> -524288 (&HFFF80000, R 255->248); RGB(255,255,255) round-trips to -460552 (&HFFF8F8F8, == #WHITE); RGB(0,100,0) -> -16752640 (&HFF006000, G 100->top-5-bits=12->96=0x60); GSPOIT(-1,-1) off-page -> 0. Confirms the RGBA5551 device truncation (8->5 at write) + 5<<3 expand (5->8 at read) and off-page = 0." }
confidence: hw_verified
related: [GPAGE, GCOLOR, RGB, RGBREAD, GSPOIT, XSCREEN, DISPLAY, VISIBLE, SPSET, BGSCREEN, COLOR]
---

# Screen & color model

The contract for `sb-render` (M2) and anything that draws.

## Screens
- Upper (top) screen: **400 × 240**. Lower (touch) screen: **320 × 240**.
- `XSCREEN` selects the screen/layer mode; `DISPLAY` selects which screen subsequent
  drawing targets; `VISIBLE` toggles per-layer visibility.

## Layer stack (back → front), composited per frame
1. **Backdrop** (solid color)
2. **GRP** graphics pages
3. **BG** tilemap layers
4. **Sprites**
5. **Console** (text)

Z-depth ordering spans **1024 (rear) … 0 (screen plane) … −256 (front)**; smaller Z draws
in front. (Exact per-layer default Z and tie-breaking: confirm vs the *composite* framebuffer
oracle — O-T6, see Open below.)

## Color

Two color representations exist and must not be confused:

| Representation | Where | Layout | Channels |
|---|---|---|---|
| **ARGB8888** | user-facing color *codes* (`RGB()`, `GSPOIT()`, the `#WHITE…` constants, `GCOLOR`) | 32-bit `(A<<24)\|(R<<16)\|(G<<8)\|B` | 8 bits each |
| **RGBA5551** | the internal **device page** pixel (what is actually stored in a GRP/sprite/BG surface) | 16-bit | R/G/B 5 bits + A 1 bit |

- `RGB(r,g,b)` / `RGB(a,r,g,b)` build an **ARGB8888** code with full 8-bit channels (it does
  NOT truncate — see `spec/instructions/RGB.yaml`, hw_verified). Truncation to 5 bits happens
  only when a code is **written into a device page**.
- Text uses a **16-color** palette (`#TBLACK`=1 … `#TWHITE`=15; see
  `spec/reference/constants.yaml`).

### RGBA5551 device-pixel bit layout (disassembled)

The 16-bit device pixel, MSB→LSB (from the pixel-read helper `FUN_00191dfc` @0x191e40):

```
bit:  15 14 13 12 11 | 10  9  8  7  6 |  5  4  3  2  1 |  0
      R4 R3 R2 R1 R0 | G4 G3 G2 G1 G0 | B4 B3 B2 B1 B0 |  A
       red (5)       |    green (5)   |    blue (5)    | alpha (1)
```

Unpack back to ARGB8888 (what `GSPOIT` returns):
- `blue8  = (pixel << 2) & 0x0000F8`    (`and r2,#0xf8 ,r0,lsl#2` — blue from pixel[5:1])
- `green8 = (pixel << 5) & 0x00F800` → byte 1 (`and r3,#0xf800,r0,lsl#5` — green from pixel[10:6])
- `red8   = (pixel << 8) & 0xF80000` → byte 2 (`and r1,#0xf80000,r0,lsl#8` — red from pixel[15:11])
- `alpha8 = (pixel & 1) ? 0xFF : 0x00`  (`tst #1` → `orrne #0xff000000`)

### 5-bit ↔ 8-bit expansion is LEFT-SHIFT ONLY

A 5-bit channel value `v` (0–31) expands to **`v << 3`** — the low 3 bits are forced to 0.
It is **NOT** the rounding form `(v<<3)|(v>>2)`. Equivalently, an 8-bit code channel is
truncated to its **top 5 bits** when stored (`b8 & 0xF8`, i.e. `(b8 >> 3) << 3`).

Evidence (all confirmed):
- **disassembled:** the unpack masks above keep only `0xF8`/`0xF800`/`0xF80000` — the low 3
  bits of each byte are masked off.
- **hw_verified (constants):** `#WHITE = &HFFF8F8F8` (each channel `0xF8 = 248 = 31<<3`,
  not `0xFF`); `#RED = &HFFF80000`, `#BLUE = &HFF0000F8`.
- **hw_verified (round-trip, sb-oracle 2026-06-22):** drawing `RGB(255,0,0)` then reading it
  back with `GSPOIT` yields `&HFFF80000` (R 255→248); `RGB(0,100,0)` → `&HFF006000`
  (G 100 → top-5-bits = 12 → `12<<3 = 96 = 0x60`); `RGB(255,255,255)` → `&HFFF8F8F8` (==`#WHITE`).

So a value drawn to a page and read back is **generally not identical** to the value written
(the documented "passed through the internal color representation" caveat). `sb-render` must
quantize on write (`& 0xF8F8F8` plus the alpha bit) and expand on read with a plain `<<3`
(implement as `expand5(v) = v << 3` / `quantize8(b) = b & 0xF8`). Off-page reads return **0**
(transparent black), not −1 (Petit Computer returned −1; SB 3.6.0 returns 0 — hw_verified).

## GRP pages
- `GRP0…GRP5` pages; `GPAGE` selects the **display** page vs the **drawing** page (you can
  draw to an off-screen page then show it). Page↔screen mapping depends on `DISPLAY`/`XSCREEN`.
- Each GRP page is a **512 × 512 RGBA5551** little-endian, row-major buffer — independent of
  `XSCREEN` mode (the visible window is a crop of it). **hw_verified** via the O-T6 GRP
  round-trip (see `spec/concepts/file-and-extdata-format.md`): the saved `GRPn:` resource is a
  28-byte `PCBN` header (`magic 'PCBN'+'0001'`, u32 LE width/height) followed by the
  512×512×2-byte pixels, captured pixel-exact.
- Graphics coordinate range on the top screen: **X 0–399, Y 0–239** (per `GFILL`/`GPSET`
  docs); the page surface itself extends to 512×512 and off-window pixels are valid storage.

## Open (→ oracle, O-T6 *composite* framebuffer — single-page GRP capture is done)
- Per-layer default Z values and the exact tie-breaking order when two layers share a Z.
- Cross-layer blending: whether the 1-bit page alpha is the only transparency, and how
  partial sprite/console alpha composites over GRP/BG (the device page itself is 1-bit alpha;
  sprite/BG color codes carry 8-bit alpha — resolve the composite rule via O-T6 composite
  capture, not the single-page GRP round-trip which is already done).
