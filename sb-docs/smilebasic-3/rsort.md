---
title: RSORT
slug: docs-sb3-rsort
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RSORT

> **Category:** Basic instructions (variables and arrays)

Sorts arrays in descending order

## Format

```sb3
RSORT [Start position, Number of elements,] Array 1 [,Array 2 , …
]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start position` | Position in Array 1 (0-) from which to start sorting |
| `Number of elements` | Number of elements in Array 1 (1-) to sort |
| `Array 1` | Array with numerical values to sort |
| `Array 2` | - Array to sort according to the result of sorting of Array 1<br>- Array 1 to Array 8 can be enumerated |

## Examples

```sb3
DIM WORK[10]
RSORT 0, 10, WORK
```
