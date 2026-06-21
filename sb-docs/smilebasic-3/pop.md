---
title: POP
slug: docs-sb3-pop
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# POP

> **Category:** Basic instructions (variables and arrays)

Removes an element from the end of an array (The number of elements will decrease by 1)

## Format

```sb3
Variable=POP( Array )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Array` | Array from which the element will be removed |

## Return Values

Value of the element that was removed

## Examples

```sb3
DIM WORK[10]
PUSH WORK, 123
A=POP(WORK)
```
