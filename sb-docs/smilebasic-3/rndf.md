---
title: RNDF
slug: docs-sb3-rndf
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RNDF

> **Category:** Mathematics

Gets a real-type random number (a real-type random number greater than 0 and less than 1.0)

## Format

```sb3
Variable = RNDF( [ Seed ID ] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Seed ID` | Random number series: 0-7 |

## Return Values

Real-type random number greater than 0 and less than 1

## Examples

```sb3
A=RNDF()
```
