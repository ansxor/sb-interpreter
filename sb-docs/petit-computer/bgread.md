---
title: BGREAD
slug: docs-ptc-bgread
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgread
content_id: 19631
created: 2023-05-16
scraped: 2026-06-21
---

# BGREAD

Read a tile from a background layer.
## Syntax
```sb
BGREAD(layer, x, y), chr, pal, h, v
BGREAD(layer, x, y), tile
BGREAD(layer, x, y), tile$
```
| Input | Description |
| --- | --- |
|`layer`| Layer to write tile to. 0 is the foreground, 1 is the background. |
|`x`| x-coordinate of tile to write |
|`y`| y-coordinate of tile to write |
| Output | Description |
| --- | --- |
|`chr`| Character id of tile |
|`pal`| Color palette of tile |
|`h`| Tile flipped horizontally |
|`v`| Tile flipped vertically |
|`tile`| Combined tile data as number |
|`tile$`| Combined tile data as hex string |

Reads a tile from the background layer and stores the tile data into one or more variables. The tile data can be read as components or as combined data - see [the overview](https://smilebasicsource.com/forum/thread/docs-page) for more info.

## Examples
```sbsyntax
'Place tile
BGPUT 0,3,6,123,4,0,1
'Read tile data into component vars
BGREAD(0,3,6),CHR,PAL,H,V
PRINT CHR,PAL,H,V
'123 4   0   1
```
```sb
'Place tile
BGPUT 0,3,6,123,4,0,1
'Read combined tile data
BGREAD(0,3,6),TILE
PRINT TILE 
'18555
```
```sb
'Place tile
BGPUT 0,3,6,123,4,0,1
'Read combined tile data
BGREAD(0,3,6),TILE$
PRINT TILE$
'487B
```

## Notes
All arguments are rounded down.

If `x` or `y` exceed the range of [0,63], only the lower six bits are used. This is equivalent to `x AND &H3F` and `y AND &H3F`.
```sb
BGREAD(0,-1,65),TILE
'equivalent to BGREAD(0,63,1),TILE
```

You can specify the same output variable more than once - it will hold the last value stored.
```sb
BGREAD(0,X,Y),A,A,B,B
'A will contain the 'pal' value of the tile at (X,Y)
'B will contain the 'v' value of the tile at (X,Y)
```

The output string `tile$` will always consist of four hex digits.

## Errors
|Action|Error|
| --- | --- |
|Two or less input arguments are specified|Syntax error|
|Four or more input arguments are specified|Missing operand|
|A string is specified for an input argument|Syntax error|
|A string is specified for an output argument|Type Mismatch|
|Zero, two, or three output arguments are specified|Missing operand|
|Five or more output arguments are specified|Missing operand|
|A value not 0 or 1 is passed for `layer`|Out of range|

## See Also
- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-bgput)
