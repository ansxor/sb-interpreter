---
title: SPCOLOR
slug: docs-sb3-spcolor
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPCOLOR

> **Category:** Sprites

## SPCOLOR (1)

Sets the display color of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPCOLOR Management number, Color code
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Color code` | 32-bit color code in the ARGB=8888 format<br>- The lower the value of A, the higher the transparency level<br>- The actual display color will be the color code multiplied by the original pixel color |

### Examples

```sb3
SPCOLOR 1,RGB(16, 255,0,0) 'A=16,R=255,G=0,B=0
```

## SPCOLOR (2)

Gets the display color of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPCOLOR Management number OUT C32
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `C32` | Variable that returns the current color code (32-bit ARGB) |

### Examples

```sb3
SPCOLOR 1 OUT C
```
