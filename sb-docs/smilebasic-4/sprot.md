---
title: SPROT
slug: docs-sb4-sprot
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-sprot
content_id: 19533
created: 2020-11-30
scraped: 2026-06-21
---

# SPROT

Set or get a sprite's rotation angle (in degrees.)

The rotation is centered at the sprite's home coordinate.

## Syntax

```sbsyntax
SPROT spriteID%, angle#
SPROT spriteID% OUT angle#
```

| Parameter | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite: 0 to 4095. |
| `angle#` | Angle to rotate the sprite by, in degrees. |

## Examples

```sb4
'rotate a sprite 45 degrees
SPROT 0,45
```

```sb4
'check the sprite's rotation angle
PRINT SPROT(0)
```
