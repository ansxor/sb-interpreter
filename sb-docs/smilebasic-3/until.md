---
title: UNTIL
slug: docs-sb3-until
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# UNTIL

> **Category:** Basic instructions (control and branching)

Repeats the process from REPEAT until the conditional expression is satisfied

- The REPEAT instruction should be placed at the beginning of the loop
- Unlike the WHILE instruction, this executes the process before determining the condition
- Exits the loop if the condition is satisfied or when the BREAK instruction is executed

## Format

```sb3
UNTIL conditional expression
```

## Conditional Expressions

| Item | Description |
| --- | --- |
| `The same conditional expressions as in IF statements can be specified` |  |
| `Comparison<br>Operators` | == Equal to<br>!= Not equal to<br>> Greater than<br>< Smaller than<br>>= Equal to or greater than<br><= Equal to or smaller than |
| `Logical operators<br>(for comparison<br>with multiple<br>conditions)` | (Condition 1 AND Condition 2) Both of the two conditions must be satisfied<br>(Condition 1 && Condition 2) Both of the two conditions must be satisfied<br>(Condition 1 OR Condition 2) Either of the two conditions must be satisfied<br>(Condition 1 \|\| Condition 2) Either of the two conditions must be satisfied<br>* The key for "\|\|" can be found on the upper left of ? on your keyboard. |

## Examples

```sb3
A=0:B=4
REPEAT
 A=A+1
UNTIL A<B
```
