---
title: GSPOIT
slug: docs-ptc-gspoit
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gspoit
content_id: 19762
created: 2025-07-27
scraped: 2026-06-21
---

# GSPOIT

Read the pixel color at selected coordinates.

## Syntax

```sbsyntax
color = GSPOIT(x, y {, page})
```

| Input | Description |
| --- | --- |
| `x` | X coordinate of pixel to read. |
| `y` | Y coordinate of pixel to read. |
| `page` | Graphics page to read from |

| Output | Description |
| --- | --- |
| `color` | Pixel color at coordinates. |

Retrieve the color from the graphics page at given coordinates. If the page is unspecified, it uses the current graphics draw page.

## Examples

```sb
GCLS 0  ' Reset background to color 0 (transparent)
X=36
Y=25
PRINT GSPOIT(X,Y)  ' Prints 0 (transparent)
GPSET X,Y,7  ' Draw a brown pixel (color 7)
PRINT GSPOIT(X,Y)  ' Prints 7 (brown)
```

```sb
GPAGE 1,1,1  ' Set draw page to 1
GPSET 0,0,6  ' Draw a cyan pixel (color 6) on page 1
GPAGE 0,0,0  ' Set draw page to 0
GCLS 0  ' Ensure page 0 is cleared
COL = GSPOIT(0,0,1)  ' Read from page 1
PRINT COL  ' Prints 6 (cyan)
```

## Notes

All arguments are rounded down.

If the coordinates `x`, `y` are outside the range of the screen, then `GSPOIT` returns -1.

```sb
PRINT GSPOIT(-1,-1)  ' Prints -1
```

## Errors

| Action | Error |
| --- | --- |
| Less than two arguments are specified | Missing operand |
| Less than three arguments are specified | Syntax error |
| A string was passed for any argument | Type Mismatch |
| A value less than 0 or greater than 3 was passed for `page` | Out of range |

## See Also

- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GCOLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-gcolor)
- [`GPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-gpset)
