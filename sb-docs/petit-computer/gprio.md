---
title: GPRIO
slug: docs-ptc-gprio
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gprio
content_id: 19755
created: 2025-04-21
scraped: 2026-06-21
---

# GPRIO

Change the display priority of the graphics page.

## Syntax
```sbsyntax
GPRIO priority
```

| Input | Description |
| --- | --- |
|`priority`| New graphics page priority.|

Changes the display priority of the graphics page. The display priority can be set differently for both screens.

## Examples
```sb
' Change graphics page priority to draw above everything.
GPRIO 0
' Print some text.
LOCATE 0,0
PRINT "Don't read this"
' Draw a line across the text using the graphics page.
GLINE 0,3,120,3,15
```
```sb
' Change graphics to display between the two BG layers
GPRIO 2
' Draw blue tiles on the background BG layer
BGFILL 1,0,1,10,1,4
' Draw red tiles on the foreground BG layer
BGFILL 0,0,0,10,0,2
' Draw a line across the BG layers
GLINE 0,0,32,32,15
```
## Notes
All arguments are rounded down.

Sprites will always display above the graphics page when they have equal priority.

```sb
' Display an arrow sprite with priority 0
SPSET 0,0,0,0,0,0
' Display graphics with priority 0
GPRIO 0
GFILL 0,0,15,15,15
```

## Errors
| Action | Error|
| --- | --- |
| More than one argument is specified | Missing operand |
| No arguments are specified | Syntax error |
| A string was passed for any argument | Type Mismatch |
| A value less than 0 or greater than 3 was passed for `priority` | Out of range |

## See Also
- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-gpage)
