---
title: CALLIDX
slug: docs-sb4-callidx
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-callidx
content_id: 19528
created: 2020-12-14
scraped: 2026-06-21
---

# CALLIDX

Get the ID associated with the current sprite or text screen callback.

During a sprite or text screen callback, `CALLIDX` will return the ID of the sprite or text screen this callback is associated with. Otherwise -1 is returned.

## Syntax

```sbsyntax
CALLIDX out id%
```

| Output | Description |
| --- | --- |
| `id%` | ID associated with current callback. If no callback is running, -1. |

## Examples

```sb4
'CALLIDX example with multiple sprites
FOR I=0 TO 9
 SPSET I,0
 SPFUNC I,"CALLBACK"
NEXT I
CALL SPRITE

DEF CALLBACK
 'prints the current sprite ID
 PRINT CALLIDX()
END
```
