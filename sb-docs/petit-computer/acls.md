---
title: ACLS
slug: docs-ptc-acls
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-acls
content_id: 19556
created: 2023-03-15
scraped: 2026-06-21
---

# ACLS

Resets most of the graphics state.

## Syntax

```sbsyntax
ACLS
```

## Examples

```sb
ACLS
```

## Notes

`ACLS` essentially combines a bunch of separate clear commands into one, such as `BGCLR`, `SPCLR`, `GCLS`, `CLS`, etc. as well as resetting colors and drawing states. `ACLS` does not reset everything, however - most notably `ACLS` does not modify any CHR resources. `ACLS` also does not reset `ICONPUSE` or the current panel type.

### Equivalent Program

The following program is a slight modification of the SmileBoom provided code that is listed under `ACLS`.

```sb
VISIBLE 1,1,1,1,1,1:ICONCLR
COLOR 0,0:CLS:GDRAWMD FALSE
FOR P=1 TO 0 STEP -1
 GPAGE P,P,P:GCOLOR 0:GCLS:GPRIO 3
 BGPAGE P:BGOFS 0,0,0:BGOFS 1,0,0
 BGCLR:BGCLIP 0,0,31,23
 SPPAGE P:SPCLR
NEXT
FOR I=0 TO 255
 COLINIT "BG", I:COLINIT "SP", I
 COLINIT "GRP",I
NEXT
```

## See Also

- [`SPCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-spclr)
- [`BGCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclr)
- [`CLS`](https://smilebasicsource.com/forum/thread/docs-ptc-cls)
- [`GCLS`](https://smilebasicsource.com/forum/thread/docs-ptc-gcls)
