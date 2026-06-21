---
title: NEXT
slug: docs-sb3-next
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# NEXT

> **Category:** Basic instructions (control and branching)

Instruction that indicates the end of a FOR loop

- See Comment for the FOR instruction for details regarding FOR to NEXT
- Using NEXT with IF in a FOR loop is not recommended
- Use CONTINUE to exit the loop before the end

## Format

```sb3
NEXT [ Control variable ]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Control variable` | - Even if a control variable is specified, it will be ignored and the instruction will work in<br>the same way as NEXT on its own<br>- Specifications such as NEXT J,I are not allowed |

## Examples

```sb3
FOR I=0 TO 9 STEP 2
 PRINT I;",";
NEXT
```
