---
title: BGCOLOR
slug: docs-sb3-bgcolor
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGCOLOR

> **Category:** BG

## BGCOLOR (1)

Sets the BG display color

### Format

```sb3
BGCOLOR Layer, Color code
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Layer number: 0-3 |
| `Color code` | - 32-bit color code in the ARGB=8888 format<br>- The RGB function is useful for this specification: RGB( R,G,B )<br>- Unlike with sprites, the alpha value is not valid (Semitransparent representation is not<br>allowed)<br>- The actual display color will be the color code multiplied by the original pixel color. |

### Examples

```sb3
BGCOLOR 1,RGB(255,0,0) 'R=255,G=0,B=0
```

## BGCOLOR (2)

Gets the BG display color

### Format

```sb3
BGCOLOR Layer OUT C32
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Layer number: 0-3 |

### Return Values

| Return Value | Description |
| --- | --- |
| `C32` | Variable that returns the current color code (32-bit ARGB) |

### Examples

```sb3
BGCOLOR 1 OUT C
```
