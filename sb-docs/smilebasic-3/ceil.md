---
title: CEIL
slug: docs-sb3-ceil
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CEIL

> **Category:** Mathematics

Gets the integer part (by rounding up to the whole number)

- The smallest integer that is not less than the specified value will be obtained
- CEIL(12.5) will be 13, while CEIL(-12.5) will be -12

## Format

```sb3
Variable = CEIL( Numerical value )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Source numerical value |

## Return Values

Integer value after rounding up

## See Also

ROUND: Round-off, FLOOR: Round-down

## Examples

```sb3
A=CEIL(12.345)
```
