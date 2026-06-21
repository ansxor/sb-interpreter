---
title: GPAGE
slug: docs-ptc-gpage
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-gpage
content_id: 19747
created: 2025-03-23
scraped: 2026-06-21
---

# GPAGE

Change the selected screen and graphics page.

## Syntax
```sbsyntax
GPAGE screen {, modify, display}
```
| Input | Description |
| --- | --- |
|`screen`| Screen to modify graphics of.|
|`modify`| Graphics page to modify.|
|`display`| Graphics page to display.|

Changes the screen graphics commands will draw to. `GPAGE 0` selects the upper screen, while `GPAGE 1` selects the lower screen. `GPAGE` can additionally change the current display GRP and select the GRP to modify. Separate display and modify pages can be selected for each screen.

## Examples
```sb
'Select upper screen for drawing
GPAGE 0
'Some graphics commands...
GLINE 0,0,100,100,11
```
```sb
' Sets the upper and lower screens to both display GRP0.
GPAGE 0,0,0
GPAGE 1,0,0
' Draw a red line once, displayed on both screens
GLINE 0,0,100,100,2

'(You can use the following line to hide the keyboard
'so the lower screen page is actually visible)
PNLTYPE "OFF":WAIT 120
```

## Notes
All arguments are rounded down.

It is possible to draw to a separate graphics page than is selected for display. The program can then swap the display and draw pages when an image is completed, hiding the process of creating the image from the user.

```sb
' Draw to page 1 and display page 0
GPAGE 0,1,0
' Some drawing commands
GLINE 0,0,100,100,2
' Swap display and draw page
GPAGE 0,0,1
```
This is most useful for slow or complex drawing operations, to remove flickering.

## Errors
| Action | Error|
| --- | --- |
| Less than one argument is specified | Missing operand |
| Exactly two arguments are specified | Missing operand |
| More than three arguments are specified | Syntax error |
| A value less than 0 or greater than 1 was passed for `screen` | Out of range |
| A value less than 0 or greater than 3 was passed for `modify` | Out of range |
| A value less than 0 or greater than 3 was passed for `display` | Out of range |
| A string was passed for any argument | Type Mismatch |

## See Also
- [Graphics overview](https://smilebasicsource.com/forum/thread/docs-ptc-graphics)
- [`GCLS`](https://smilebasicsource.com/forum/thread/docs-ptc-gcls)
