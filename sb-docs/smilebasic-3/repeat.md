---
title: REPEAT
slug: docs-sb3-repeat
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# REPEAT

> **Category:** Basic instructions (control and branching)

Instruction for starting a REPEAT loop

- The UNTIL instruction and a conditional expression should be placed at the end of the loop
- Unlike the WHILE instruction, this executes the process before determining the condition
- Exits the loop when the condition is satisfied or when the BREAK instruction is executed

## Format

```sb3
REPEAT
```

## Examples

```sb3
A=0:B=4
REPEAT
 A=A+1
UNTIL A>B
```
