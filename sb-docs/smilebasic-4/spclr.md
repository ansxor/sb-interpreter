---
title: SPCLR
slug: docs-sb4-spclr
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spclr
content_id: 19535
created: 2020-11-30
scraped: 2026-06-21
---

# SPCLR

Clear sprites.

## Clear All

All sprites are cleared.
This does not affect the GSPRITE (sprite 4095.)

```sbsyntax
SPCLR
```

## Clear Sprite

The given sprite is cleared.

```sbsyntax
SPCLR spriteID%
```

| Input | Description |
| --- | --- |
| `spriteID%` | The sprite to clear. |

## Clear Range

All sprites set in the given ID range are cleared.

```sbsyntax
SPCLR startID%, endID%
```

| Input | Description |
| --- | --- |
| `startID%` | A range of sprite IDs to clear. |
| `endID%` | A range of sprite IDs to clear. |

## Examples

```sb4
'clear sprite 0
SPCLR 0
```

```sb4
'clear all sprites
SPCLR
```

```sb4
'clear sprites 100 to 200
SPCLR 100,200
```
