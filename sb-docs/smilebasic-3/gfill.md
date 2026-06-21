---
title: GFILL
slug: docs-sb3-gfill
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GFILL

> **Category:** Graphics

Draws a quadrangle on the graphic screen and fills it with a color

## Format

```sb3
GFILL Start point X,Start point Y, End point X,End point Y [,Color code]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start point X,Y` | Start point coordinates (X: 0-399, Y: 0-239) |
| `End point X,Y` | End point coordinates (X: 0-399, Y: 0-239) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Examples

```sb3
GFILL 0,0,399,239
```
