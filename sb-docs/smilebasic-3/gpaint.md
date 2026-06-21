---
title: GPAINT
slug: docs-sb3-gpaint
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GPAINT

> **Category:** Graphics

Fills the graphic screen with color If the border color is omitted, the color range at the start point coordinates will be used

## Format

```sb3
GPAINT Start point X, Start point Y [ ,Fill Color [, Border color ] ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start point X,Y` | Coordinates to start filling from (X: 0-399, Y: 0-239) |
| `Fill color` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |
| `Border color` | Should be specified in the same way as Fill color |

## Examples

```sb3
GPAINT 200,120,RGB(255,0,0),RGB(0,0,0)
```
