---
title: FOR
slug: docs-sb3-for
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# FOR

> **Category:** Basic instructions (control and branching)

Repeats the process for the specified number of times

- The NEXT instruction should be placed at the end of the process
- If the condition is not satisfied, the process may not be executed at all

## Format

```sb3
FOR Loop variable=Initial value TO End value [STEP Increment]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Loop variable` | Variable for loop count (On each iteration of the loop, the increment is added to the count) |
| `Initial value` | Value or expression for the loop variable at the start of the loop |
| `TO End value` | Value or expression for the loop variable at the end of the loop |
| `STEP Increment` | - Increment added to the loop variable at the end of the loop (If omitted, 1)<br>- If the increment is specified as a fractional value, the intended loop count may not be<br>achieved due to operational errors. |

## Examples

```sb3
FOR I=0 TO 9 STEP 2
 PRINT I;",";
NEXT
```
