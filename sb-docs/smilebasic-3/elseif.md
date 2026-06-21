---
title: ELSEIF
slug: docs-sb3-elseif
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ELSEIF

> **Category:** Basic instructions (control and branching)

Additional conditional evaluation if the IF condition is not satisfied

- Used to evaluate another condition if the IF condition is not satisfied
- See Comment for IF for details regarding conditional evaluation

## Format

```sb3
IF Conditional expression THEN Process to execute when the condition is satisfied ELSEIF Conditional expression
THEN Process to execute when the condition is satisfied ENDIF
```

## Examples

```sb3
IF A==1 THEN
 PRINT "Congratulations":BEEP 0
ELSEIF A==2 THEN
 PRINT "Bad luck"
ELSE IF A==3 THEN
 PRINT "So-so"
ENDIF '--- Required when using ELSE IF
ENDIF
```
