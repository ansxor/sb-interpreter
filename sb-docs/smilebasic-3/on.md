---
title: ON
slug: docs-sb3-on
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# ON

> **Category:** Basic instructions (control and branching)

## ON (1)

Branches to a label line according to the control variable value

- The branch number begins with 0, unlike in conventional BASIC

### Format

```sb3
ON Control variable GOTO @Label 0, @Label 1 …
```

### Arguments

| Argument | Description |
| --- | --- |
| `@Label 0` | Jump target to use when the control variable is 0 |
| `@Label 1` | Jump target to use when the control variable is 1<br>:<br>- Prepare the necessary number of branch destinations<br>- Label strings cannot be used in the ON to GOTO labels |

### Examples

```sb3
ON IDX GOTO @JMP_A,@JMP_B
PRINT "OVER":END
@JMP_A
PRINT "IDX=0":END
@JMP_B
PRINT "IDX=1":END
```

## ON (2)

Calls a sub-routine according to a control variable value

- The branch number begins with 0, unlike in conventional BASIC

### Format

```sb3
ON Control variable GOSUB @Label 0, @Label 1 …
```

### Arguments

| Argument | Description |
| --- | --- |
| `@Label 0` | Sub-routine when the control variable is 0 |
| `@Label 1` | Sub-routine when the control variable is 1<br>:<br>- Prepare the necessary number of branch destinations<br>- Label strings cannot be used in the ON to GOSUB labels |

### Examples

```sb3
ON IDX GOSUB @SUB_A,@SUB_B
PRINT "EXIT":END
@SUB_A
PRINT "IDX=0":RETURN
@SUB_B
PRINT "IDX=1":RETURN
```
