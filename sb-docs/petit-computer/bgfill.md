---
title: BGFILL
slug: docs-ptc-bgfill
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgfill
content_id: 19628
created: 2023-05-14
scraped: 2026-06-21
---

# BGFILL

Fill a rectangular region of a background layer.

## Syntax

```sbsyntax
BGFILL layer, x1, y1, x2, y2, chr, pal, h, v
BGFILL layer, x1, y1, x2, y2, tile
BGFILL layer, x1, y1, x2, y2, tile$
```

| Input | Description |
| --- | --- |
| `layer` | Layer to write tile to. 0 is the foreground, 1 is the background. |
| `x1` | x-coordinate of first corner of region to write |
| `y1` | y-coordinate of first corner of region to write |
| `x2` | x-coordinate of second corner of region to write |
| `y2` | y-coordinate of second corner of region to write |
| `chr` | Character id of tile |
| `pal` | Color palette of tile |
| `h` | Flips tile horizontally if set |
| `v` | Flips tile vertically if set |
| `tile` | Combined tile data as number |
| `tile$` | Combined tile data as hex string |

Fills a region of the selected background layer with the specified tile. The region is defined as the rectangle with corners at (`x1`,`y1`) and (`x2`,`y2`). The tile's data can be specified as components or as combined data - see [the overview](https://smilebasicsource.com/forum/thread/docs-ptc-background) for more info.

## Examples

```sb
CHR=37 'grass tile
PAL=8 'palette 8 (brown and green)
'Fills half of the screen with grass tiles.
BGFILL 0,0,0,15,23,CHR,PAL,0,0
```

```sb
TILE=1 'gray square
'Fills the entire background layer with gray tiles.
BGFILL 1,0,0,63,63,TILE
```

```sb
'chr=526 hex$(chr)=&H20E 
TILE$="020E" 'downward pointing arrow
'Fills the visible portion of the foreground layer with arrows
BGFILL 0,0,0,31,23,TILE$
```

## Notes

All numeric arguments are rounded down.

If `tile` is greater than 65535 (2^16-1), only the lower 16 bits are used. This is equivalent to `tile AND &HFFFF` or `tile % 65536`

If `x1`,`y1`,`x2`, or `y2` are out of the range [0,63], the values will be clipped to that range.

The order of coordinates used for `x1`,`y1` and `x2`,`y2` does not matter.

```sb
'These will cover the same region of tiles:
BGFILL 1,0,0,3,3,2
WAIT 60
BGFILL 0,3,3,0,0,3
```

## Errors

| Action | Error |
| --- | --- |
| Zero to five arguments are specified | Missing operand |
| Seven to Eight arguments are specified | Missing operand |
| Ten or more arguments are specified | Syntax error |
| A value not 0 or 1 is passed for `layer` | Out of range |
| A value not in range of 0 to 1023 is used for `chr` | Out of range |
| A value not 0 or 1 is used for `h` or `v` | Out of range |
| A value not in range of 0 to 15 is used for `pal` | Out of range |
| A string value is passed in place of a numeric argument | Type Mismatch |
| A string that isn't exactly four hex digits is used for `tile$` | Illegal function call |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclr)
- [`BGPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-bgput)
