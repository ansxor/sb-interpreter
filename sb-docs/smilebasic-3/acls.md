---
title: ACLS
slug: docs-sb3-acls
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ACLS

> **Category:** Screen control

Resets the draw settings to their settings when BASIC was started

- The same operations as those shown after END in the Examples should be executed
- Sound settings such as BGM will not be affected

## Format

```sb3
ACLS
```

## Examples

```sb3
ACLS
END
'---
XSCREEN 0
LOAD "GRP4:SYS/DEFSP.GRP"
LOAD "GRP5:SYS/DEFBG.GRP"
FONTDEF
SPDEF
DISPLAY 1
WIDTH 8
BACKCOLOR 0
FADE 0
COLOR 15,0:LOCATE 0,0,0:ATTR 0:CLS
GPAGE 1,1:SPPAGE 4:BGPAGE 5
VISIBLE 1,1,1,1
DISPLAY 0
BACKCOLOR 0
FADE 0
WIDTH 8
COLOR 15,0:LOCATE 0,0,0:ATTR 0:CLS
FOR I=0 TO 3:GPAGE I,I:GCLS 0:NEXT
GPAGE 0,0:GPRIO 1024
SPPAGE 4:SPCLR
BGPAGE 5:BGCLR
VISIBLE 1,1,1,1
```
