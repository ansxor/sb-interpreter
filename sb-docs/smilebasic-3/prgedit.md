---
title: PRGEDIT
slug: docs-sb3-prgedit
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGEDIT

> **Category:** Source code manipulation

Specifies the program SLOT to manipulate, and the current line

## Format

```sb3
PRGEDIT Program SLOT [,Line number]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Program SLOT` | - Program SLOT to manipulate: 0-3<br>- Specifying the SLOT currently running will give an error |
| `Line number` | - Line to manipulate (Current line)<br>- If this is omitted, the first line will be the current line<br>- If -1 is specified for the line number, the current line will be the last line |

## Examples

```sb3
PRGEDIT 0
```
