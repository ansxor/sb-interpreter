---
title: COLOR
slug: docs-ptc-color
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-color
content_id: 19567
created: 2023-03-22
scraped: 2026-06-21
---

# COLOR

Set the text color palette.
## Syntax
```sbsyntax
COLOR fgcolor{,bgcolor}
```
|Input | Description|
| --- | --- |
|fgcolor| Console text foreground color palette. Affects both screens. |
|bgcolor| Console text background color palette. Only affects top screen, optional. |

`COLOR` sets the foreground and optionally the background color palettes for the text screens. The foreground color affects text printed by both `PRINT` and `PNLSTR`, though `PNLSTR`'s optional color argument overrides this.

The background color argument only applies to the top screen. If it is omitted, the previous value is kept. The background color argument disables the background tile when bgcolor=0; otherwise this sets the color palette of the background tile.

## Examples
```sb
' Set the text color to red
COLOR 13
PRINT "Red text!"
```
```sb
' Set the text color to orange and background to brown
COLOR 7,8
PRINT "Orange with brown background!"
' Set the text color to yellow. Background is kept brown.
COLOR 3
PRINT "Yellow with brown background!"
```
```sb
' White text with transparent background
COLOR 0,0
PRINT "This is the default."
```
```sb
' Comparison of top screen and bottom screen coloring
PNLTYPE "OFF"
COLOR 3,5
' Uses COLOR, has background
PRINT "Yellow on green!"
' Uses COLOR, has no background
PNLSTR 0,0,"Just yellow."
' Uses optional color argument, has no background
PNLSTR 0,1,"Just green.",5
' To keep the panel disabled, wait so you can see the effects.
WAIT 300
```

## Notes
All arguments are rounded down.

The values set by `COLOR` persist until `COLOR` is called again or `ACLS` is used, which resets both foreground and background colors to zero. The background tile used is tile 15 of BGD0U when bgcolor is nonzero.

While `COLOR` always affects both screens, you can override this by always specifying the optional color argument of `PNLSTR`. However, if you are printing a lot of lower screen text with one color it can be faster to set `COLOR` once instead of using the optional argument of `PNLSTR` repeatedly.

The color palettes used for the text screens are COL0U for the upper screen and COL0L for the lower screen.

## Errors
|Action | Error |
| --- | --- |
| No arguments are provided | Missing operand |
| Three or more arguments are provided | Syntax error |
| A string argument is provided | Type Mismatch |
| An argument is less than 0 or more than 15 after rounding | Out of range |

## See Also
- Console overview
- `PNLSTR`
