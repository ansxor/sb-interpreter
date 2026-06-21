---
title: SPOFS
slug: docs-sb4-spofs
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spofs
content_id: 19531
created: 2020-11-30
scraped: 2026-06-21
---

# SPOFS

Set or get a sprite's position on screen.

## Syntax

```sbsyntax
SPOFS spriteID%, x%, y% {, z% }
SPOFS spriteID% OUT x%, y% {, z% }
```

| Parameter | Description |
| --- | --- |
| `spriteID%` | The ID of the target sprite. |
| `x%` | X,Y coordinates of the sprite, in display pixels.<br>When setting, these parameters may be empty. If empty the coordinate value is unchanged. |
| `y%` | X,Y coordinates of the sprite, in display pixels.<br>When setting, these parameters may be empty. If empty the coordinate value is unchanged. |
| `z%` | Display priority of the sprite: -4095 to 4095 (optional.)<br>Lower values increase display priority. |

## Examples

```sb4
'put sprite 0 at 100,100
SPOFS 0,100,100
```

```sb4
'check position of sprite 0
SPOFS 0 OUT X,Y,Z
PRINT X,Y,Z
```

```sb4
'change only the Y coordinate
SPOFS 0, ,200
```

```sb4
'change only the Z coordinate
SPOFS 0, , ,10
```
