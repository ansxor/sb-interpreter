---
title: GLINE
slug: docs-ptc-gline
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gline
content_id: 19745
created: 2025-03-23
scraped: 2026-06-21
---

# GLINE

Draw a line between two points. 

## Syntax
```sbsyntax
GLINE x1, y1, x2, y2 {, color}
```
| Input | Description |
| --- | --- |
|`x1`| First point x-coordinate|
|`y1`| First point y-coordinate|
|`x2`| Second point x-coordinate|
|`y2`| Second point y-coordinate|
|`color`| Color of line|

Draws a line of pixels between the two given points on the currently selected graphics page, using the specified `color`.
If `color` is omitted, uses the current graphics draw color as set by `GCOLOR`.

## Examples
```sb
' Draw a red line from the upper left corner to the bottom right.
GLINE 0,0,255,191,2
```
```sb
' Draw a blue line from the lower left corner to the upper right.
GCOLOR 4
GLINE 0,191,255,0
```

## Notes
All arguments are rounded down.

Coordinates specified off of the edge of the screen are valid. Only the portion of the line visible on screen is drawn - no errors occur.

```sb
' Draw a yellow line through the middle of the screen, with coordinates starting off the edges of the screen.
GLINE -50,96,300,96,12
```
## Errors
| Action | Error|
| --- | --- |
| Less than three arguments are specified | Missing operand |
| More than six arguments are specified | Syntax error |
| A value less than 0 or greater than 255 was passed for `color` | Out of range |
| A string was passed for any argument | Type Mismatch |

## See Also
- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-gpage)
- [`GCOLOR`](https://smilebasicsource.com/forum/thread/docs-ptc-gcolor)
- [`GCLS`](https://smilebasicsource.com/forum/thread/docs-ptc-gcls)
