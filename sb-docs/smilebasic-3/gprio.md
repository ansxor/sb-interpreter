---
title: GPRIO
slug: docs-sb3-gprio
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# GPRIO

> **Category:** Graphics

Changes the display order of the graphic screen If 3D mode is used, the whole graphic screen will be affected

## Format

```sb3
GPRIO Z-coordinate
```

## Arguments

| Argument | Description |
| --- | --- |
| `Z-coordinate` | Coordinate in the depth direction (Rear:1024 < Screen surface:0 < Front:-256) |

## Examples

```sb3
GPRIO -100
```
