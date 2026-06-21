---
title: GPSET
slug: docs-sb3-gpset
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GPSET

> **Category:** Graphics

Puts a pixel on the graphic screen

## Format

```sb3
GPSET X-coordinate,Y-coordinate [,Color code ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `X-,Y-coordinates` | Coordinates to place the pixel at |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Examples

```sb3
GPSET 100,50
```
