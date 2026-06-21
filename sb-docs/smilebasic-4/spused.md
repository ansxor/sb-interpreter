---
title: SPUSED
slug: docs-sb4-spused
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spused
content_id: 19537
created: 2020-12-14
scraped: 2026-06-21
---

# SPUSED

Check if a sprite ID is in use.

## Syntax

```sbsyntax
SPUSED spriteID% OUT used%
```

| Input | Description |
| --- | --- |
| `spriteID%` | The sprite ID to check. |

| Output | Description |
| --- | --- |
| `used%` | `#TRUE` if the sprite ID is in use, `#FALSE` otherwise. |

## Examples

```sb4
SPSET 0,0
PRINT SPUSED(0)
PRINT SPUSED(1)
```
