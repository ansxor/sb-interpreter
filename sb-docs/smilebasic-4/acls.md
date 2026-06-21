---
title: ACLS
slug: docs-sb4-acls
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-acls
content_id: 19526
created: 2020-05-16
scraped: 2026-06-21
---

# ACLS

Completely clear and reset the display state.

## Syntax

```sbsyntax
ACLS { keepGRP%, keepSPDEF%, keepFont% {, keepANIMDEF% }}
```

| Input | Description |
| --- | --- |
| `keepGRP%` | Optional; default false. If false, all graphics pages (except the font page) will be cleared, and the default sprite sheet will be loaded on page 4 |
| `keepSPDEF%` | Optional; default false. If false, the default sprite template definitions will be loaded. |
| `keepFont%` | Optional; default false. If false, the default font will be loaded to page 5 |
| `keepANIMDEF%` | Optional; default false. If false, sprite animation definitions will be cleared |

## Examples

```sb4
'completely reset the display
ACLS
```

```sb4
'reset screen while keeping all resources (fast!)
ACLS #TRUE, #TRUE, #TRUE, #TRUE
```

## Notes

### Equivalent Program

SmileBoom lists the following program as being equivalent to `ACLS`.

```sb4
'ACLS works almost the same as the following program
DEF ACLS KEEPGRP,KEEPSPDEF,KEEPGRPF
 VAR SCW=400,SCH=240

 XSCREEN SCW,SCH,2
 SPCLR
 BACKCOLOR &HFF000000
 FOR I=0 TO 3
  TSCREEN I,16,16:TPAGE I,4,1024,0
  TLAYER I,0:TOFS I,0,0,0:ATTR I,0
  COLOR I,#C_WHITE:CLS I
 NEXT
 TSCREEN #TCONSOLE,16,8:TPAGE #CONSOLE,4,1024,0
 TLAYER #TCONSOLE, 0:TOFS #TCONSOLE,0,0,-4095:ATTR 0
 COLOR #C_WHITE:CLS
 IF !KEEPGRP THEN
  FOR I=0 TO 5:GTARGET I:GCLS:NEXT
  LOADG "GRP:#SYS/DEFGRP",4
 ENDIF
 IF !KEEPGRPF THEN
  LOADG "GRP:#SYS/DEFFONT",#GRPF
 ENDIF
 GTARGET 0
 GCOLOR #C_WHITE
 GCLIP 0,0,#GRPWIDTH-1,#GRPHEIGHT-1
 SPSET #GSPRITE,0,0,SCW,SCH,0,1
 SPPAGE #GSPRITE,0
 SPLAYER #GSPRITE,0
 SPOFS #GSPRITE,0,0,4095
 IF !KEEPSPDEF THEN
  SPDEF
 ENDIF
 ANIMDEF
 FOR I=0 TO 7:LAYER I:LFILTER I:LCLIP I:LMATRIX I:NEXT
 FADE 0
END
```

### Using Keep Flags

The keep flags can be used in situations where you need to completely reset the display, without erasing currently loaded GRPs, sprite templates, etc. This could be used in games where a scene transition is needed, for example. An additional benefit to these flags is to improve start speed of a program. Normal `ACLS` is slow because the initial settings must be loaded from files. The default GRPs are the greatest contributor to the slowdown. In some situations it can be appropriate to use the keep flags and do manual resetting of certain properties to improve the startup time of your program.

| Command | Description | Time (sec.) |
| --- | --- | --- |
| `ACLS 0,0,0` | clear all | 0.24 |
| `ACLS 1,0,0` | don't clear GRPs | 0.10 |
| `ACLS 0,0,1` | don't clear font | 0.15 |
| `ACLS 1,0,1` | keep font+GRPs | 0.02 |

## See Also

- `XSCREEN`
- `CLS`
- `GCLS`
