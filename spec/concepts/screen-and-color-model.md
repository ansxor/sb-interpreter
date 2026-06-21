---
title: Screen & color model
slug: screen-and-color-model
area: graphics
kind: concept
sources:
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/screen-layout.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/manual/computer-colors-rgb.md" }
  - { type: documented, ref: "sb-docs/smilebasic-3/reference/constants.md" }
  - { type: disassembled, ref: "constant #WHITE=&HFFF8F8F8 implies 5-bit<<3 expansion", confidence: hypothesis }
confidence: documented
related: [GPAGE, GCOLOR, RGB, XSCREEN, DISPLAY, VISIBLE, SPSET, BGSCREEN, COLOR]
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
in front. (Exact per-layer default Z and tie-breaking: confirm vs oracle framebuffer — O-T6.)

## Color
- Graphics use **RGBA5551** (15 color bits → **32768** colors; 1 alpha bit). Text uses a
  **16-color** palette (`#TBLACK`=1 … `#TWHITE`=15; see `spec/reference/constants.yaml`).
- User-facing color values (e.g. from `RGB()`, the `#WHITE`… constants) are **ARGB8888**.
- **5-bit → 8-bit expansion is left-shift only (low 3 bits zero), NOT `(v<<3)|(v>>2)`.**
  Evidence: `#WHITE = &HFFF8F8F8`, whose channel `0xF8 = 248 = 31<<3` (would be `0xFF` under
  rounding expansion). Implemented in `sb-render::expand5`. ⚠ confirm the full ramp + the
  exact internal framebuffer format against the oracle framebuffer (O-T6) before calling
  this `hw_verified` (queued in HARVEST_QUEUE.md).

## GRP pages
- `GRP0…GRP5` pages; `GPAGE` selects the **display** page vs the **drawing** page (you can
  draw to an off-screen page then show it). Page↔screen mapping depends on `DISPLAY`/`XSCREEN`.
- Graphics coordinate range on the top screen: **X 0–399, Y 0–239** (per `GFILL`/`GPSET` docs).

## Open (→ oracle)
- Exact internal framebuffer pixel format + tiling, per-layer Z defaults, additive/alpha
  blending rules between layers. Resolve via O-T6 framebuffer capture.
