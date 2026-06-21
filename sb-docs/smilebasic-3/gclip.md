---
title: GCLIP
slug: docs-sb3-gclip
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GCLIP

> **Category:** Graphics

Specifies a clipping area on the graphic screen

- When the range is omitted in display mode, the whole screen will be clipped
- When the range is omitted in write mode, the whole graphic page is assumed

## Format

```sb3
GCLIP Clip mode [,Start point X,Start point Y,End point X, End point Y]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Clip mode` | 0: Clipping for display, 1: Clipping for writing |
| `Start point X,Y` | Start point coordinates for the clipping area |
| `End point X,Y` | End point coordinates for the clipping area |

## Examples

```sb3
GCLIP 0,100,100,200,200
```
