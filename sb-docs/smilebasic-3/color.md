---
title: COLOR
slug: docs-sb3-color
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# COLOR

> **Category:** Console input/output

Specifies the display colors for the console screen Constants for text colors are available (#TBLACK to #TWHITE)

## Format

```sb3
COLOR Drawing color [,Background color]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Drawing color` | 0: Transparent color<br>1: Black, #TBLACK<br>2: Dark red, #TMAROON<br>3: Red, #TRED<br>4: Dark green, #TGREEN<br>5: Green, #TLIME<br>6: Dark yellow, #TOLIVE<br>7: Yellow, #TYELLOW<br>8: Dark blue, #TNAVY<br>9: Blue, #TBLUE<br>10: Dark magenta, #TPURPLE<br>11: Magenta, #TMAGENTA<br>12: Dark cyan, #TTEAL<br>13: Cyan, #TCYAN<br>14: Gray, #TGRAY<br>15: White, #TWHITE |
| `Background color` | - Background color number for each character (0-15: See the drawing colors)<br>- If only the background color needs to be changed, the drawing color can be omitted |

## Examples

```sb3
COLOR 7,4
COLOR #TWHITE
COLOR ,0
```
