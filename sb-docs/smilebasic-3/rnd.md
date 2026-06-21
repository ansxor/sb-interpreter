---
title: RND
slug: docs-sb3-rnd
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RND

> **Category:** Mathematics

Gets an integer random number (0 - the maximum value minus 1)

## Format

```sb3
Variable = RND( [ Seed ID, ] Maximum value )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Seed ID` | Random number series: 0-7 |
| `Maximum value` | Upper limit of the random number to be obtained |

## Return Values

Integer random number in the range 0 - the maximum value minus 1

## Examples

```sb3
A=RND(100)
```
