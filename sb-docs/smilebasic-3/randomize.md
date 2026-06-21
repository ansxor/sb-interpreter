---
title: RANDOMIZE
slug: docs-sb3-randomize
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RANDOMIZE

> **Category:** Mathematics

Initializes a random number series

## Format

```sb3
RANDOMIZE Seed ID [, Seed value ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Seed ID` | Random number series type: 0-7 |
| `Seed value` | If 0 or omitted, initialization will be performed using available entropy information |

## Examples

```sb3
RANDOMIZE 0
```
