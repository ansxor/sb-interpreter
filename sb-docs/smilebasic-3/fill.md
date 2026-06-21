---
title: FILL
slug: docs-sb3-fill
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# FILL

> **Category:** Basic instructions (variables and arrays)

Sets all the elements in an array to the specified value

- Partial changes can also be made by specifying an offset and number of elements
- You can specify any type of array, including integer, real number, or string

## Format

```sb3
FILL Array, Value [,Offset [,Number of elements]]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Array` | The array that you want to overwrite with a value |
| `Value` | The desired number or string |
| `Offset` | The position to begin writing the value from |
| `Number of elements` | The number of elements to write the value into |

## Examples

```sb3
DIM WORK[10]
FILL WORK,0
```
