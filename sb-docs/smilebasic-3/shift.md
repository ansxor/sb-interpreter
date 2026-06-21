---
title: SHIFT
slug: docs-sb3-shift
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SHIFT

> **Category:** Basic instructions (variables and arrays)

Removes an element from the start of an array (The number of elements will decrease by 1)

## Format

```sb3
Variable=SHIFT( Array )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Array` | Array from which the element will be removed |

## Examples

```sb3
DIM WORK[10]
UNSHIFT WORK, 123
A=SHIFT(WORK)
```
