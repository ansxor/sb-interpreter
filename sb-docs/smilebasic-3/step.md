---
title: STEP
slug: docs-sb3-step
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# STEP

> **Category:** Basic instructions (control and branching)

Specifies the increment value for a FOR loop count

- See Comment for the FOR instruction for details regarding FOR to NEXT

## Format

```sb3
STEP Increment
```

## Examples

```sb3
FOR I=0 TO 9 STEP 2
 PRINT I;",";
NEXT
```
