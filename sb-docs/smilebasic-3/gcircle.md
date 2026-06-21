---
title: GCIRCLE
slug: docs-sb3-gcircle
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# GCIRCLE

> **Category:** Graphics

## GCIRCLE (1)

Draws a circle on the graphic screen

### Format

```sb3
GCIRCLE Center point X,Center point Y, Radius [,Color code ]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Center point X,Y` | Center point coordinates (X: 0-399, Y: 0-239) |
| `Radius` | Radius of the circle (in pixels) 1- |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

### Examples

```sb3
GCIRCLE 200,120,30
```

## GCIRCLE (2)

Draws an arc on the graphic screen

### Format

```sb3
GCIRCLE Center point X, Center point Y, Radius, Start angle, End angle [ Flag [ Color code ]]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Center point X,Y` | Center point coordinates (X: 0-399, Y: 0-239) |
| `Radius` | Radius of the circle (in pixels) 1- |
| `Start angle, End<br>angle` | Angle of the arc 0-360 |
| `Flag` | Drawing method (0=Arc, 1=Sector) |
| `Color code` | Color code consisting of an 8-bit value for each ARGB element * See GCOLOR |

### Examples

```sb3
GCIRCLE 200,120,30, 0,45, 1
```
