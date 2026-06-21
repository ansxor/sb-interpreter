---
title: BGCOPY
slug: docs-ptc-bgcopy
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgcopy
content_id: 19632
created: 2023-05-17
scraped: 2026-06-21
---

# BGCOPY

Copy a region within a BG layer.

## Syntax

```sbsyntax
BGCOPY layer, startx, starty, endx, endy, transferx, transfery
```

| Input | Description |
| --- | --- |
| `layer` | Layer to read and write |
| `startx` | Starting x-coordinate of region to copy |
| `starty` | Starting y-coordinate of region to copy |
| `endx` | Ending x-coordinate of region to copy |
| `endy` | Ending y-coordinate of region to copy |
| `transferx` | Destination starting x-coordinate to copy to |
| `transfery` | Destination starting y-coordinate to copy to |

Copies the rectangular region with corners (`startx`,`starty`) and (`endx`, `endy`) to the destination (`transferx`,`transfery`). If the regions overlap, the copy will still work, taking the original rectangle and copying it fully to the new destination.

## Examples

```sb
' Set up some tiles to copy
FOR I=0 TO 15
FOR J=0 TO 3
BGPUT 0,I,J,I+32*J
NEXT
NEXT
' Copy tiles starting from (0,0) and ending at (15,3) to destination (0,12)
BGCOPY 0,0,0,15,3,0,12
```

## Notes

All arguments are rounded down.

The rectangular region's corners can be defined in any way. For example,

```sb
'All of the following are equivalent
'Rectangle between (0,0) and (15,3)
BGCOPY 0,0,0,15,3,0,12
BGCOPY 0,15,3,0,0,0,12
'Rectangle between (15,0) and (0,3)
BGCOPY 0,15,0,0,3,0,12
BGCOPY 0,0,3,15,0,0,12
```

The destination (`transferx`,`transfery`) is always the destination of the upper-left tile of the rectangular region, regardless of the order of the corners in the source rectangle.

If the destination region would extend past the edge of the background layer, tiles that would exceed the range are not copied.

If the source region extends past the edge of the background layer, tiles beyond the edge are copied as tile zero (the transparent tile).

If the destination coordinates are negative, the behavior of the destination region is strange. The copy destination essentially becomes (`ABS(transferx)`,`ABS(transfery)`), but with the region from (0,0) to the upper edge and left edge of the copied region zeroed out.

```sb
'Set up source region
FOR I=0 TO 15
FOR J=0 TO 7
BGPUT 0,I,J,I+32*J
NEXT
NEXT
'Copy to negative coordinates
BGCOPY 0,0,0,15,7,-2,1
```

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are passed | Syntax error |
| One to six arguments are passed | Missing operand |
| Eight or more arguments are passed | Missing operand |
| A value not 0 or 1 is passed for `layer` | Out of range |
| A string value is passed | Type Mismatch |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-bgput)
- [`BGREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-bgread)
