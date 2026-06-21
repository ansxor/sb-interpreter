---
title: SPCLIP
slug: docs-sb3-spclip
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPCLIP

> **Category:** Sprites

Specifies a clipping area in the sprite

- If the range is omitted, the whole screen will be assumed

## Format

```sb3
SPCLIP [Start point X,Start point Y,End point X, End point Y]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start point X,Y` | Start point coordinates for the clipping area (X: 0-399, Y: 0-239) |
| `End point X,Y` | End point coordinates for the clipping area (X: 0-399, Y: 0-239) |

## Examples

```sb3
SPCLIP 100,100,200,200
```
