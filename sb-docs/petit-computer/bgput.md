---
title: BGPUT
slug: docs-ptc-bgput
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgput
content_id: 19627
created: 2023-05-12
scraped: 2026-06-21
---

# BGPUT

Place a tile on a background layer.
## Syntax
```sbsyntax
BGPUT layer, x, y, chr, pal, h, v
BGPUT layer, x, y, tile
BGPUT layer, x, y, tile$
```
| Input | Description |
| --- | --- |
|`layer`| Layer to write tile to. 0 is the foreground, 1 is the background. |
|`x`| x-coordinate of tile to write |
|`y`| y-coordinate of tile to write |
|`chr`| Character id of tile |
|`pal`| Color palette of tile |
|`h`| Flips tile horizontally if set |
|`v`| Flips tile vertically if set |
|`tile`| Combined tile data as number |
|`tile$`| Combined tile data as hex string |

Writes a tile to the specified background layer `layer` at coordinates `x`,`y`. The tile's data can be specified as components or as combined data - see [the overview](https://smilebasicsource.com/forum/thread/docs-page) for more info

## Examples
```sb
CHR=214 'upper left of plant tiles
PAL=8 'palette 8 (brown and green)
'Draws a 16x16 plant using 8x8 BG tiles on the background layer
BGPUT 1,15,7,CHR,PAL,0,0
BGPUT 1,16,7,CHR+1,PAL,0,0
BGPUT 1,15,8,CHR+32,PAL,0,0
BGPUT 1,16,8,CHR+33,PAL,0,0
```
```sb
'Draws many background tiles to the foreground layer
PAL=9*4096 'palette 9 (darker brown and green)
FOR I=0 TO 31
 FOR J=0 TO 7
  BGPUT 0,I,J,I+32*J+PAL
 NEXT
NEXT
'Note that if you run this after the first example,
'the background layer's tiles are hidden behind
'the foreground layer's tiles.
```
```sb
'Draws a 24x8 platform using three different BG tiles
'Note that "1AC" is the combined character ID and h/v
'and "7" is the palette. Here, h and v are both zero.
BGPUT 0,20,10,"71AC"
BGPUT 0,20,10,"71AD"
BGPUT 0,20,10,"71AE"

```
## Notes
All numeric arguments are rounded down.

If `tile` is greater than 65535 (2^16-1), only the lower 16 bits are used. This is equivalent to `tile AND &HFFFF` or `tile % 65536`

If `x` or `y` are greater than 63, only the lower six bits are used. This is equivalent to `x AND &H3F` or `x % 64`, and similar for `y`.

You can not omit the `h` or `v` arguments while having separate `chr` and `pal` arguments. You must specify either all separate arguments or one combined argument. However, you can use the combined form to just specify `chr`, which will treat `pal`,`h`, and `v` as zero.
```sb
'The following are equivalent
BGPUT 0,0,0,123
BGPUT 0,0,0,123,0,0,0
'You aren't allowed to do this, though:
'BGPUT 0,0,0,123,0
```

## Errors
|Action | Error|
| --- | --- |
|Zero to three arguments are specified | Missing operand |
|Five to six arguments are specified | Missing operand |
|Eight or more arguments are specified | Syntax error |
|A value not 0 or 1 is passed for `layer` | Out of range |
|A value not in range of 0 to 1023 is used for `chr` | Out of range |
|A value not 0 or 1 is used for `h` or `v` | Out of range |
|A value not in range of 0 to 15 is used for `pal` | Out of range |
|A string value is passed in place of a numeric argument | Type Mismatch |
|A string that isn't exactly four hex digits is used for `tile$` | Illegal function call |

## See Also
- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclr)
- [`BGREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-bgread)
