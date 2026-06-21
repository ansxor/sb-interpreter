---
title: PRGSIZE
slug: docs-sb3-prgsize
system: SmileBASIC 3
type: command
category: Source code manipulation
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRGSIZE

> **Category:** Source code manipulation

Gets the number of lines in the source code

## Format

```sb3
Variable=PRGSIZE( [Program SLOT [,Type of value to get]] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Program SLOT` | Program SLOT from which to get the number of lines: 0-3 |
| `Type of value to<br>get` | 0 = Number of lines, 1 = number of characters, 2 = number of free characters (Default: 0) |

## Return Values

Type-appropriate value

## Examples

```sb3
A=PRGSIZE(0)
```
