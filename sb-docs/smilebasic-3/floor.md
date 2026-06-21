---
title: FLOOR
slug: docs-sb3-floor
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# FLOOR

> **Category:** Mathematics

Gets the integer part (by rounding down to the whole number)

- The largest integer that is not greater than the specified value will be obtained
- FLOOR(12.5) will be 12, while FLOOR(-12.5) will be -13

## Format

```sb3
Variable = FLOOR( Numerical value )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Source numerical value |

## Return Values

Integer value after rounding down

## See Also

ROUND: Round-off, CEIL: Round-up

## Examples

```sb3
A=FLOOR(12.345)
```
