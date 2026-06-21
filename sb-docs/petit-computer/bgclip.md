---
title: BGCLIP
slug: docs-ptc-bgclip
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgclip
content_id: 19633
created: 2023-05-18
scraped: 2026-06-21
---

# BGCLIP

Set the visible portion of the background layers.

## Syntax

```sbsyntax
BGCLIP x1, y1, x2, y2
```

| Input | Description |
| --- | --- |
| `x1` | Starting x-coordinate of clipping region, in tiles |
| `y1` | Starting y-coordinate of clipping region, in tiles |
| `x2` | Ending x-coordinate of clipping region, in tiles |
| `y2` | Ending y-coordinate of clipping region, in tiles |

Sets the clipping region of the background layers for the current screen. The region is the rectangle from the corner defined by (`x1`,`y1`) to the corner defined by (`x2`,`y2`), moving right and down.

## Examples

```sb
' Fill visible portion of screen with gray
BGFILL 0,0,0,31,23,1
' Restrict display to left half of screen
BGCLIP 0,0,15,23
```

## Notes

All arguments are rounded down.

`BGCLIP` only affects one screen at a time, and affects both layers at once. This is different from every other BG command, which usually operate on layers.

Unlike most other BG commands, the order of coordinates for `BGCLIP` matters. The region will always be defined starting from (`x1`,`y1`) moving right and down to (`x2`,`y2`). This allows the region to wrap around the edge of the screen.

```sb
' Fill visible portion of screen with gray
BGFILL 0,0,0,31,23,1
' Only show 3x3 tiles in each corner of the screen
BGCLIP 29,21,2,2
```

In cases where the coordinates wrap around and the ending coordinate is one less than the starting coordinate, the background layers will be completely invisible. Only one of the x or y coordinates needs to have this property.

```sb
' Fill visible portion of screen with gray
BGFILL 0,0,0,31,23,1
' Entirely hide the screen
BGCLIP 0,1,0,0
' Another way to hide screen
BGCLIP 1,0,0,0
```

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are passed | Syntax error |
| Between one and three arguments are passed | Missing operand |
| Five or more arguments are passed | Missing operand |
| A string is passed | Type Mismatch |
| A value less than zero or greater than 31 is passed for `x1` or `x2` | Out of range |
| A value less than zero or greater than 23 is passed for `y1` or `y2` | Out of range |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`VISIBLE`](https://smilebasicsource.com/forum/thread/docs-ptc-visible)
