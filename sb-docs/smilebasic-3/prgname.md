---
title: PRGNAME$
slug: docs-sb3-prgname
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGNAME$

> **Category:** Source code manipulation

Program file name File that has been handled with the LOAD/SAVE instruction

## Format

```sb3
String variable=PRGNAME$([Program SLOT])
```

## Arguments

| Argument | Description |
| --- | --- |
| `Program SLOT` | Program SLOT from which to get the file name: 0-3 |

## Return Values

- Program file name
- When a program is running, the SLOT in which it is running
- When no program is running, the "SLOT of the last program run"
- The "SLOT of the last program run" is usually SLOT 0
- If a running program has been suspended with the STOP instruction or the START button, or if an error has

occurred, the SLOT at that time will be the “SLOT of the last program run” and will remain so until the next RUN

## Examples

```sb3
PRINT PRGNAME$(0)
```
