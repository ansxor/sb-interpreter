---
title: GTRI
slug: docs-sb3-gtri
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GTRI

> **Category:** Graphics

Draws a triangle on the graphic screen and fills it with a color

## Format

```sb3
GTRI X1,Y1, X2,Y2, X3,Y3 [,Color code]
```

## Arguments

| Argument | Description |
| --- | --- |
| `X1,Y1` | Vertex 1(X: 0-399, Y: 0-239) |
| `X2,Y2` | Vertex 2 (X: 0-399, Y: 0-239) |
| `X3,Y3` | Vertex 3 (X: 0-399, Y: 0-239) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Examples

```sb3
GTRI 200,10,300,200,100,200
```
