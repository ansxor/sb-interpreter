---
title: RGBREAD
slug: docs-sb3-rgbread
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RGBREAD

> **Category:** Graphics

Gets each RGB element from a color code

## Format

```sb3
RGBREAD Color code OUT [A,] R,G,B
```

## Arguments

| Argument | Description |
| --- | --- |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

## Return Values

| Return Value | Description |
| --- | --- |
| `A` | Variable to receive transparency information (Opaque: 255 - 0: Transparent) |
| `R,G,B` | Variables to receive 8-bit color information (each 0-255) |

## Examples

```sb3
RGBREAD C OUT R,G,B
```
