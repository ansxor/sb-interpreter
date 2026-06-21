---
title: SUBRUN
slug: docs-sb4-subrun
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-subrun
content_id: 19462
created: 2020-04-29
scraped: 2026-06-21
---

# SUBRUN

Load and run a program in the sub environment. Direct Mode only.

## Syntax

```sbsyntax
SUBRUN program$
```

| Input | Output |
| --- | --- |
| `program$` | The path to the program file to run. |

## Examples

```sb4
'run the graphic editor subprogram
SUBRUN "#TOOL/GAHAKU.PRG"
'run the program MYTOOL in the current project as a subprogram
SUBRUN "MYTOOL"
```

## Notes

- The program is always loaded and run in *slot 0* in the sub environment.
- If a subprogram is currently running, it will be stopped.
- The memory of subprogram slot 0 is cleared (including variables and `SPFUNC`/`TFUNC` mappings) before this program is loaded.
