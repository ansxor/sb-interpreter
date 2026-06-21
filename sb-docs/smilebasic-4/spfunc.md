---
title: SPFUNC
slug: docs-sb4-spfunc
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spfunc
content_id: 19536
created: 2020-12-14
scraped: 2026-06-21
---

# SPFUNC

Assign a callback to a sprite.

A callback can be assigned using either a label or a user-defined function name. If a label is used, the callback works like `GOSUB`. If a function is used, it works like `CALL`. A callback cannot take any arguments or return any values.

A sprite can only have one callback at a time. All sprite callbacks are called in batch by `CALL SPRITE`, in order of sprite ID. During a callback, `CALLIDX` returns the ID of the associated sprite.

## Set

```sbsyntax
SPFUNC spriteID%, callback$
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. |
| `callback$` | A string containing the name of the callback. Can be a label or a user-defined function.<br>The name can also start with a slot number, e.g. `0:@FOO` or `1:BAR` to specify a label or function in a specific slot. |

## Clear

Remove the callback from the sprite.

```sbsyntax
SPFUNC spriteID%
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. |

## Examples

```sb4
'use a DEF as a callback
SPFUNC 0,"CALLBACK"
CALL SPRITE

DEF CALLBACK
 'prints the current sprite ID
 PRINT CALLIDX()
END
```

```sb4
'use a label as a callback
SPFUNC 0,@CALLBACK
CALL SPRITE
END

@CALLBACK
 'prints the current sprite ID
 PRINT CALLIDX()
RETURN
```

```sb4
'clear a callback
SPFUNC 0
CALL SPRITE  'nothing happens!
```

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
