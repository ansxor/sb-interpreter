---
title: BREAK
slug: docs-sb3-break
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BREAK

> **Category:** Basic instructions (control and branching)

Forces a loop to end

- Used in FOR ... NEXT, WHILE ... WEND, REPEAT ... UNTIL

## Format

```sb3
BREAK
```

## Examples

```sb3
FOR I=0 TO 9
 IF I==1 THEN CONTINUE
 IF I==7 THEN BREAK
 PRINT I;",";
NEXT
```
