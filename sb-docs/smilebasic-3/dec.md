---
title: DEC
slug: docs-sb3-dec
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# DEC

> **Category:** Basic instructions (variables and arrays)

Decrements the value of a variable by -1 If the Expression argument is specified, the value of the expression will be subtracted

## Format

```sb3
DEC Variable [, Expression ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Variable` | Name of variable to decrement value of |
| `Expression` | Value to subtract (If omitted, 1) |

## Examples

```sb3
DEC X
DEC X,3
```
