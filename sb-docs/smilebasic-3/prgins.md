---
title: PRGINS
slug: docs-sb3-prgins
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGINS

> **Category:** Source code manipulation

Inserts a line in the current line For a character string including the line feed code CHR$(10), multiple lines will be inserted

## Format

```sb3
PRGINS "Character string" [,Flag]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Source character string to insert |
| `Flag` | 1 = Inserts a line after the current line<br>0 = Inserts a line before the current line (If omitted = 0, before the current line) |

## Examples

```sb3
PRGINS "PRINT "+CHR$(34)+"HELLO"+CHR$(34)
```
