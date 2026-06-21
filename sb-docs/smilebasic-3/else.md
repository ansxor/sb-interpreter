---
title: ELSE
slug: docs-sb3-else
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ELSE

> **Category:** Basic instructions (control and branching)

Control destination if the IF condition is not satisfied See Comment for IF for details regarding conditional evaluation

## Format

```sb3
IF Conditional expression THEN Process to execute when the condition is satisfied ELSE Process to execute when the
condition is not satisfied [ENDIF]
```

## Examples

```sb3
IF A==1 THEN PRINT "OK"
IF A<1 THEN @JMP1 ELSE PRINT DATE$
IF A==1 THEN
 PRINT "Congratulations":BEEP 72
ELSE
 PRINT "Bad luck"
ENDIF
@JMP1
END
```
