---
title: GOTO
slug: docs-sb3-goto
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# GOTO

> **Category:** Basic instructions (control and branching)

## GOTO (1)

Forces branching

### Format

```sb3
GOTO @Label
```

### Arguments

| Argument | Description |
| --- | --- |
| `@Label` | - Jump target @Label name<br>- Label string, which is the Label name enclosed in "" (String variables are also allowed)<br>- A program SLOT can be specified in the following format: "1:@Label name"<br>- The target SLOT should be enabled beforehand with the USE instruction |

### Examples

```sb3
GOTO @MAIN
JP$="@MAIN":GOTO JP$
```

## GOTO (2)

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

## GOTO (3)

Branches to @Label if the condition is satisfied

- See Comment for IF for details regarding conditional evaluation

### Format

```sb3
IF Conditional expression GOTO @Label [ELSE Process to execute when the condition is not satisfied]
```

### Notes when using a string for the label

```
- Label strings can also be used for the label
- Label strings are not allowed if GOTO immediately after ELSE is omitted
× IF A==0 GOTO "@LABEL1" ELSE "@LABEL2"
○
 IF A==0 GOTO "@LABEL1" ELSE @LABEL2
○
 IF A==0 GOTO "@LABEL1" ELSE GOTO "@LABEL2"
```

### Examples

```sb3
IF A==1 GOTO @MAIN
IF X>0 GOTO @JMP1 ELSE PRINT A$
IF Y==5 GOTO @JMP1 ELSE @JMP2
@JMP1
PRINT "@JMP1"
@JMP2
PRINT "@JMP2"
END
```
