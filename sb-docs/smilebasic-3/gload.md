---
title: GLOAD
slug: docs-sb3-gload
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# GLOAD

> **Category:** Graphics

## GLOAD (1)

Copies image data from an array to the graphic screen

### Format

```sb3
GLOAD [X,Y,Width,Height,] Image array,Color conversion flag,Copy mode
```

### Arguments

| Argument | Description |
| --- | --- |
| `X,Y,Width,Height` | Start point X-coordinate, start point Y-coordinate, and width/height (in pixels) of the copy<br>destination range |
| `Image array` | Numerical value array containing image data stored with GSAVE |
| `Color conversion<br>flag` | 0: Performs color conversion (Converts to 32-bit logical colors)<br>1: Leaves the physical codes as they are (16-bit) |
| `Copy mode` | TRUE = Copies the transparent color, FALSE = Does not copy the transparent color |

### Examples

```sb3
GLOAD 0,0,512,512, WORK, 1, 0
```

## GLOAD (2)

Copies image data from an array to the graphic screen Colors will be handled as index colors from palettes

### Format

```sb3
GLOAD [X,Y,Width,Height,] Image array,Palette array,Copy mode
```

### Arguments

| Argument | Description |
| --- | --- |
| `X,Y,Width,Height` | Start point X-coordinate, start point Y-coordinate, and width/height (in pixels) of the copy<br>destination range |
| `Image array` | Numerical value array containing image data stored with GSAVE |
| `Palette array` | Numerical value array containing palette data |
| `Copy mode` | TRUE = Copies the transparent color, FALSE = Does not copy the transparent color |

### Examples

```sb3
GLOAD 0,0,512,512, WORK, PALETTE, 0
```
