---
title: SPHOME
slug: docs-sb4-sphome
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-sphome
content_id: 19532
created: 2020-11-30
scraped: 2026-06-21
---

# SPHOME

Set or get a sprite's home coordinate (transform origin.)

The home coordinate is used as the origin (center point) for `SPOFS`, `SPROT`, and `SPSCALE`. Additionally, the home coordinate is used as the origin for sprite collision, and the start point of the default collision box in `SPCOL` (if a collision box is not specified.) The home coordinate is specified relative to the sprite's upper-left corner, e.g. the center of a sprite is `width/2, height/2`.

If the sprite was set using a definition template, its default home coordinate is set by the template. Otherwise, it is `0,0`, or the sprite's upper-left corner.

## Syntax

```sbsyntax
SPHOME spriteID%, homeX%, homeY%
SPHOME spriteID% OUT homeX%, homeY%
```

| Parameter | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite: 0 to 4095. |
| `homeX%` | Home coordinate X,Y of the sprite. |
| `homeY%` | Home coordinate X,Y of the sprite. |

## Examples

```sb4
'set the home coordinate
SPHOME 0,8,8
```

```sb4
'get the home coordinate
SPHOME 0 OUT HX,HY
PRINT HX,HY
```
