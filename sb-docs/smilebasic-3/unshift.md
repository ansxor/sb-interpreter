---
title: UNSHIFT
slug: docs-sb3-unshift
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# UNSHIFT

> **Category:** Basic instructions (variables and arrays)

Adds an element to the start of an array (The number of elements will increase by 1)

## Format

```sb3
UNSHIFT Array, Expression
```

## Arguments

| Argument | Description |
| --- | --- |
| `Array` | Array to which the element will be added |
| `Expression` | Value of the element to add |

## Examples

```sb3
DIM WORK[10]
UNSHIFT WORK, 123
```
