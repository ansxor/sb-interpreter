---
title: SPSCALE
slug: docs-sb4-spscale
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spscale
content_id: 19534
created: 2020-11-30
scraped: 2026-06-21
---

# SPSCALE

Set or get a sprite's scale factor.

The scaling is centered at the sprite's home coordinate.

## Syntax

```sbsyntax
SPSCALE spriteID%, scaleX#, scaleY#
SPSCALE spriteID% OUT scaleX#, scaleY#
```

| Parameter | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite: 0 to 4095. |
| `scaleX#` | Scale factor along the X and Y axes, as a real number.<br>1.0 = no scale, 2.0 = 200%/double scale, 0.5 = 50%/half scale, etc. |
| `scaleY#` | Scale factor along the X and Y axes, as a real number.<br>1.0 = no scale, 2.0 = 200%/double scale, 0.5 = 50%/half scale, etc. |

## Examples

```sb4
'set the sprite's scale
SPSCALE 0,2,1
```

```sb4
'get the sprite's scale
SPSCALE 0 OUT SX,SY
PRINT SX,SY
```
