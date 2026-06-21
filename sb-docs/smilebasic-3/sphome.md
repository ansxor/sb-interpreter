---
title: SPHOME
slug: docs-sb3-sphome
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPHOME

> **Category:** Sprites

## SPHOME (1)

Specifies the reference point (home position) for the coordinates of a sprite

- Position reference point for the SPOFS instruction
- Center point for rotation and scaling
- Center coordinates for collision detection
- If used before SPSET, an error will occur

### Format

```sb3
SPHOME Management number,Position X,Position Y
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite for which to set the reference point: 0-511 |
| `Position X,Y` | Relative coordinates with the top left corner of the sprite as the origin (0,0) |

### Examples

```sb3
SPHOME 34,16,16
```

## SPHOME (2)

Gets the reference point (home position) for the coordinates of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPHOME Management number OUT HX,HY
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `HX,HY` | Variable to receive the coordinates of the reference point |

### Examples

```sb3
SPHOME 10 OUT HX,HY
```
