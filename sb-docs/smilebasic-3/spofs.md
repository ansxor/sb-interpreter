---
title: SPOFS
slug: docs-sb3-spofs
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPOFS

> **Category:** Sprites

## SPOFS (1)

Changes (moves) the coordinates of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPOFS Management number, X, Y [,Z]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `X,Y` | Screen coordinates where the sprite will be displayed |
| `Z` | Coordinate in the depth direction (Rear:1024 < Screen surface:0 < Front:-256) |

### Examples

```sb3
SPOFS 23,50,80
SPOFS 23,,,1000
SPOFS 23,150,180,0
```

## SPOFS (2)

Gets the coordinates of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPOFS Management number OUT X,Y[,Z]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `X,Y` | Variable to receive the coordinates |
| `Z` | Variable to receive the depth information |

### Examples

```sb3
SPOFS 12 OUT X,Y,Z
```
