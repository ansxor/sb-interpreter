---
title: GPSET
slug: docs-ptc-gpset
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gpset
content_id: 19746
created: 2025-03-23
scraped: 2026-06-21
---

# GPSET

Put a pixel on the graphics draw page.

## Syntax

```sbsyntax
GPSET x, y {, color}
```

| Input | Description |
| --- | --- |
| `x` | X-coordinate of the pixel. |
| `y` | Y-coordinate of the pixel. |
| `color` | Color index of the pixel. |

Places a pixel at the specified coordinates on the graphics draw page. If the color of the pixel is unspecified the current graphics draw color as set by [`GCOLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-gcolor) is used.

## Examples

```sb
'Place a red pixel near the center of the screen.
GPSET 128,96,2
```

```sb
'Place a blue pixel near the center of the screen.
GCOLOR 4
GPSET 128,96
```

## Notes

All arguments are rounded down.

If coordinates are specified outside of the normal range, no error occurs.

```sb
'Attempts to write a pixel off the edge of the screen.
'No error will occur, but no pixel can be seen.
GPSET 300,200,5
```

## Errors

| Action | Error |
| --- | --- |
| Less than two arguments are specified | Missing operand |
| More than three arguments are specified | Syntax error |
| A value less than zero or greater than 255 is passed for `color` | Out of range |
| A string is passed for any argument | Type Mismatch |

## See Also

- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-gpage)
- [`GCLS`](https://smilebasicsource.com/forum/thread/docs-ptc-gcls)
- [`GCOLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-gcolor)
