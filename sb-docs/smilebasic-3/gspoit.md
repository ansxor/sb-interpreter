---
title: GSPOIT
slug: docs-sb3-gspoit
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GSPOIT

> **Category:** Graphics

Gets a color from the specified coordinates on the graphic screen The return value may not be the same as the value specified at the time of drawing because it has passed through the internal color representation

## Format

```sb3
Variable = GSPOIT( X-coordinate,Y-coordinate )
```

## Arguments

| Argument | Description |
| --- | --- |
| `X-,Y-coordinates` | Coordinates of which to get the color (X: 0-399, Y: 0-239) |

## Return Values

Color code consisting of an 8-bit value for each ARGB element * See GCOLOR

## Examples

```sb3
C=GSPOIT(100,100)
```
