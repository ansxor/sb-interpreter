---
title: SPSCALE
slug: docs-sb3-spscale
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPSCALE

> **Category:** Sprites

## SPSCALE (1)

Changes the scale (display magnification) of a sprite

- For collision detection that takes scale into account, SPCOL should first be executed

If used before SPSET, an error will occur

### Format

```sb3
SPSCALE Management number, Magnification X, Magnification Y
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Magnification X,Y` | 0.5 (50%) - 1.0 (100%) - 2.0 (200%) - |

### Examples

```sb3
SPSCALE 56, 0.75, 0.75
```

## SPSCALE (2)

Gets the display magnification of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPSCALE Management number OUT SX,SY
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `SX,SY` | Variable to receive the magnification |

### Examples

```sb3
SPSCALE 45 OUT SX,SY
```
