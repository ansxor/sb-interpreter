---
title: ENDIF
slug: docs-sb3-endif
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ENDIF

> **Category:** Basic instructions (control and branching)

Ends if processing spans multiple lines after control switching with IF See Comment for IF for details regarding conditional evaluation

## Format

```sb3
IF Conditional expression THEN Process to execute when the condition is satisfied ELSE Process to execute when the
condition is not satisfied [ENDIF]
```

## Examples

```sb3
IF A==0 THEN
 PRINT "A=0"
ENDIF
```
