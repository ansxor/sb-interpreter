---
title: IF
slug: docs-sb3-if
system: SmileBASIC 3
type: command
category: Basic instructions (control and branching)
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# IF

> **Category:** Basic instructions (control and branching)

## IF (1)

Executes Process 1 if the condition is satisfied, or Process 2 if the condition is not satisfied

- GOTO can be omitted immediately after THEN or ELSE
- ENDIF should be used when the process spans multiple lines

### Format

```sb3
IF Conditional expression THEN Process to execute when the condition is satisfied [ELSE Process to execute when
the condition is not satisfied] [ENDIF]
```

### Conditional Expressions

| Item | Description |
| --- | --- |
| `Comparison<br>Operators` | == Equal to<br>!= Not equal to<br>> Greater than<br>< Smaller than<br>>= Equal to or greater than<br><= Equal to or smaller than |
| `Logical Operators<br>(for comparing<br>multiple<br>conditions)` | (Condition 1 AND Condition 2) Both of the conditions should be satisfied<br>(Condition 1 && Condition 2) Both of the conditions should be satisfied<br>(Condition 1 OR Condition 2) Either one of the conditions should be satisfied<br>(Condition 1 \|\| Condition 2) Either one of the conditions should be satisfied<br>* The key for the "\|\|" characters is located to the upper left of the "?" key on the keyboard. |

### Examples

```sb3
IF A==1 THEN PRINT "OK"
IF A>1 THEN @JMP1 ELSE PRINT DATE$
IF A==1 THEN
 PRINT "Congratulations":BEEP 72
ELSE
 PRINT "Bad luck"
ENDIF
@JMP1
END
```

## IF (2)

Branches to @Label if the condition is satisfied See Comment for IF for details regarding conditional evaluation

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
