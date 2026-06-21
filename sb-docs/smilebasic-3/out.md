---
title: OUT
slug: docs-sb3-out
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# OUT

> **Category:** Basic instructions (control and branching)

Instruction used when multiple outputs are required

- Used to declare a DEF instruction that returns multiple values
- Also used in built-in instructions that return multiple values

## Format

```sb3
OUT
```

## Examples

```sb3
DEF SUB A OUT D,M
 D=A DIV 10
 M=A MOD 10
END
SUB 34 OUT DV,ML
PRINT DV,ML
```
