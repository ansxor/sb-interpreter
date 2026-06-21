---
title: CLASSIFY
slug: docs-sb3-classify
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CLASSIFY

> **Category:** Mathematics

Determines whether a given number is an ordinary numerical value, infinity, or not-a-number (NaN)

## Format

```sb3
Variable = CLASSIFY( Numerical value )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Real number to check |

## Return Values

```
0 = Ordinary numerical value, 1 = Infinity, 2 = NaN
```

## Examples

```sb3
A=CLASSIFY(0.5)
```
