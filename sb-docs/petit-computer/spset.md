---
title: SPSET
slug: docs-ptc-spset
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spset
content_id: 19639
created: 2023-05-28
scraped: 2026-06-21
---

# SPSET

Create a sprite.

## Syntax

```sbsyntax
SPSET id, chr, pal, h, v, prio {, width, height}
```

| Input | Description |
| --- | --- |
| `id` | Sprite ID to use |
| `chr` | Sprite character to start with |
| `pal` | Color palette to use for sprite |
| `h` | Horizontally flips sprite if value is one |
| `v` | Vertically flips sprite if value is one |
| `prio` | Graphics draw priority |
| `width` | Sprite width in pixels. Default 16 if not specified. |
| `height` | Sprite height in pixels. Default 16 if not specified. |

Creates a sprite with the given information on the current screen.

## Examples

```sb
'Create a sprite of a boy using
'character 64, color palette 2
SPSET 0,64,2,0,0,0
```

```sb
'Create sprites of size 8x8, 16x16, 32x32, and 64x64
FOR I=0 TO 3
 SIZE=POW(2,I+3)
 SPSET I,64,2,0,0,0,8,8
 SPOFS I,64*I
NEXT
```

## Notes

All arguments are rounded down.

Sprite will be initialized at (0,0) with scale 100 and angle 0. Sprite variables are initialized to zero. The sprite's origin will be the upper left of the sprite. The sprite will not be animated.

Valid combinations of `width`, `height` are:

| | 8 | 16 | 32 | 64 |
| --- | --- | --- | --- | --- |
| 8 | OK | OK | OK | NO |
| 16 | OK | OK | OK | NO |
| 32 | OK | OK | OK | OK |
| 64 | NO | NO | OK | OK |

Any other pairs of values will cause an `Illegal function call` error.

At most 100 sprites can be created per screen, with ids 0-99.

If too many large sprites are in use at the same time, parts of some sprites or the graphics page can fail to render. Rotated sprites and scaled sprites also have a much higher rendering cost. This is a hardware limitation.

A sprite can be re-created with `SPSET` if necessary. This is the only way to change the sprite's size. However, this will also reset various other sprite properties, such as position, scaling, and rotations, so in most cases it is better to use `SPCHR`.

## Errors

| Action | Error |
| --- | --- |
| Less than six arguments are specified | Missing operand |
| Seven arguments are specified | Missing operand |
| Nine or more arguments are specified | Syntax Error |
| A value less than zero or greater than 99 is passed for `id` | Out of range |
| A value less than zero or greater than 511 is passed for `chr` on the top screen | Out of range |
| A value less than zero or greater than 117 is passed for `chr` on the bottom screen | Out of range |
| A value less than zero or greater than 15 is passed for `pal` | Out of range |
| A value not zero or one is passed for `h` | Out of range |
| A value not zero or one is passed for `v` | Out of range |
| A value less than zero or greater than three is passed for `prio` | Out of range |
| An invalid pair of parameters is passed for `width` and `height` | Illegal function call |
| A string is passed for any argument | Type Mismatch |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-spclr)
- [`SPCHR`](https://smilebasicsource.com/forum/thread/docs-ptc-spchr)
