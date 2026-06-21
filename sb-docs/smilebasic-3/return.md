---
title: RETURN
slug: docs-sb3-return
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# RETURN

> **Category:** Basic instructions (control and branching)

## RETURN (1)

Returns from a sub-routine to the caller

### Format

```sb3
RETURN
```

### Examples

```sb3
RETURN
```

## RETURN (2)

Returns a value from a sub-routine while returning to the caller Used to return values in a DEF instruction defined as function type

### Format

```sb3
RETURN
```

### Examples

```sb3
DEF CALC(A,B)
 RETURN A*B
END
PRINT CALC(2,3)
```
