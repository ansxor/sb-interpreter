---
title: GCOLOR
slug: docs-sb3-gcolor
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# GCOLOR

> **Category:** Graphics

## GCOLOR (1)

Specifies the graphic draw color

### Format

```sb3
GCOLOR Color code
```

### Arguments

| Argument | Description |
| --- | --- |
| `Color code` | - Usually specified with the RGB function, e.g., GCOLOR RGB(64,255,48)<br>- To specify a numerical value directly, a color code consisting of an 8-bit value for each<br>ARGB element should be specified<br>- An 8-bit value for A (255: Opaque, Otherwise: Transparent) + one for each RGB element (0-<br>255) |

### Examples

```sb3
GCOLOR RGB(255,0,0)
```

## GCOLOR (2)

Specifies the graphic draw color

### Format

```sb3
GCOLOR OUT C32
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `C32` | Color code consisting of an 8-bit value for each ARGB element |

### Examples

```sb3
GCOLOR OUT C32
```
