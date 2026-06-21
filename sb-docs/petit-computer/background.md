---
title: Background overview
slug: docs-ptc-background
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-background
content_id: 19602
created: 2023-04-13
scraped: 2026-06-21
---

# Background overview

## Summary

Each screen of the system has two background layers (sometimes abbreviated as BG layers) available for use. These background layers provide a basic tile map system. Tiles can be placed on each layer, and each layer can be scrolled independently. Layers can also be saved as or loaded from `SCR` resources.

## Interface

### Commands

| [`BGCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclr) | Clears the background layer(s) specified. |
| --- | --- |
| [`BGPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-bgpage) | Sets the current screen to modify the background layers of. |
| [`BGPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-bgput) | Places a tile on a background layer. |
| [`BGFILL`](https://smilebasicsource.com/forum/thread/docs-ptc-bgfill) | Fills a rectangular region of a background layer with a tile. |
| [`BGOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-bgofs) | Scrolls a background layer. |
| [`BGREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-bgread) | Reads a tile from a background layer. |
| [`BGCOPY`](https://smilebasicsource.com/forum/thread/docs-ptc-bgcopy) | Copies a region of tiles to another part of the layer. |
| [`BGCLIP`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclip) | Sets clipping boundaries for both background layers. |

### Functions

[`BGCHK`](https://smilebasicsource.com/forum/thread/docs-ptc-bgchk) — Gets the current animation state of a background layer.

### Resources

| `BGU0U`-`BGU3U` | `CHR` tile resources for the upper screen. |
| --- | --- |
| `BGU0L`-`BGU3L` | `CHR` tile resources for the lower screen. |
| `SCU0U`-`SCU1U` | `SCR` resources for the upper screen. |
| `SCU0L`-`SCU1L` | `SCR` resources for the lower screen. |
| `COL0U` | `COL` resource for upper screen tiles. Shared with the text console. |
| `COL0L` | `COL` resource for lower screen tiles. Shared with the lower screen text console and panel background. |

## Additional Information

Each background layer is a 64x64 grid of 4-bit color 8x8 pixel tiles. Tile resources are shared between both layers, but are separate by screen. Layer 0 is the foreground layer and layer 1 is the background layer - there is no way to switch the order of these layers.

Each tile's data is stored in a 16-bit number. The format for a single tile's data is broken down into bits as follows:

```none
 PPPPVHTT TTTTTTTT
15      8 7      0
```

Within this value, the components are

- `P` is 4 bits [0-15] representing the color palette of the tile
- `V` is 1 bit [0-1] representing if the tile is flipped vertically
- `H` is 1 bit [0-1] representing if the tile is flipped horizontally
- `T` is 10 bits [0-1023] representing the tile character to use.

Every BG command that accepts or returns a tile value has three forms:

- one that takes each component as separate arguments
- one that accepts combined data as a number
- one that accepts combined data as a 4-digit hexadecimal string.

For example, `BGPUT` can be used in each of these ways:

```
X=5:Y=7
' components
TILE=32
H=0
V=1
PAL=9
' combined (number)
TD=TILE+H*1024+V*2048+PAL*4096
' combined (string)
TD$=HEX$(TD,4)

' all three of these will put the same tile
BGPUT 0,X,Y,TILE,PAL,H,V
BGPUT 0,X+1,Y,TD
BGPUT 0,X+2,Y,TD$
```
