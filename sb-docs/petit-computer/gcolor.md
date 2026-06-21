---
title: GCOLOR
slug: docs-ptc-gcolor
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gcolor
content_id: 19761
created: 2025-07-27
scraped: 2026-06-21
---

# GCOLOR

Set the default graphics color.

## Syntax
```sbsyntax
GCOLOR color
```
| Input | Description |
| --- | --- |
|`color`| The new default graphics color to use.|

Sets the default graphics color for future draw commands, such as `GPSET`, `GLINE`, etc. This color is used when a graphics command does not specify the color explicitly.

## Examples
```sb
GCOLOR 3  ' Set default color to pink
GLINE 0,0,99,99  ' Draws a pink line (use default color)
GLINE 0,99,99,0,4  ' Draws a blue line (color was specified)
```

## Notes
All arguments are rounded down.

There is no built-in function to read the currently set default color.

## Errors
| Action | Error|
| --- | --- |
| More than one argument is specified | Missing operand |
| Less than one argument is specified | Syntax error |
| A string was passed for any argument | Type Mismatch |
| A value less than 0 or greater than 255 was passed for `color` | Out of range |

## See Also
- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-gpset)
- [`GSPOIT`](https://smilebasicsource.com/forum/thread/docs-ptc-gspoit)
