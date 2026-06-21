---
title: CONT
slug: docs-sb3-cont
system: SmileBASIC 3
type: command
category: Instructions available only in DIRECT mode
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CONT

> **Category:** Instructions available only in DIRECT mode

Resumes a suspended program

- DIRECT mode only
- Execution is resumed from the location it was suspended at using the START button, the STOP instruction, or due

to an error

- If the program has been stopped and then edited, it cannot be resumed
- If the program was suspended while waiting for user input, it cannot be resumed
- The program may not be able to be resumed depending on the type of error that occurred

## Format

```sb3
CONT
```

## Examples

```sb3
CONT
```
