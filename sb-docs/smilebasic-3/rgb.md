---
title: RGB
slug: docs-sb3-rgb
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RGB

> **Category:** Graphics

Gets a color code based on 8-bit RGB values

- Black RGB(0,0,0)
- White RGB(255,255,255)
- Light gray RGB(224,224,224)
- Gray RGB(128,128,128)
- Dark gray RGB(64,64,64)
- Red RGB(255,0,0)
- Pink RGB(255,96,208)
- Purple RGB(160,32,255)
- Light blue RGB(80,208,255)
- Blue RGB(0,32,255)
- Yellow green RGB(96,255,128)
- Green RGB(0,192,0)
- Yellow RGB(255,224,32)
- Orange RGB(255,160,16)
- Brown RGB(160,128,96)
- Pale pink RGB(255,208,160)

## Format

```sb3
Variable = RGB( [Transparency,] Red,Green,Blue )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Transparency` | - Transparency information (255: Opaque, Otherwise: Transparent)<br>- A transparency level in the range 0-255 can be specified for SPCOLOR |
| `Red, Green, Blue` | Each color has an 8-bit color tone value (each 0-255) |

## Return Values

```
Variable=Color code (An 8-bit value for each ARGB element) * See GCOLOR
```

## Examples

```sb3
GPSET 0,0, RGB(255,255,0) 'YELLOW
```
