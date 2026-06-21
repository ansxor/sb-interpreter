---
title: GLINE
slug: docs-sb3-gline
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GLINE

> **Category:** Graphics

Draws a straight line on the graphic screen

## Format

```sb3
GLINE Start point X,Start point Y, End point X,End point Y [,Color code ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start point X,Y` | Start point coordinates (X: 0-399, Y: 0-239) |
| `End point X,Y` | End point coordinates (X: 0-399, Y: 0-239) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Examples

```sb3
GLINE 0,0,399,239,RGB(0,255,255)
```
