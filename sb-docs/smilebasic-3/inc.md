---
title: INC
slug: docs-sb3-inc
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# INC

> **Category:** Basic instructions (variables and arrays)

Increments the value of a variable by +1 If the Expression argument is specified, the value of the expression will be added

## Format

```sb3
INC Variable [, Expression ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Variable` | Name of variable to increment value of |
| `Expression` | Value to add (If omitted, 1) |

## Examples

```sb3
INC X
INC X,3
```
