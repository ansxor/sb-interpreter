---
title: DEF
slug: docs-sb3-def
system: SmileBASIC 3
type: command
category: Basic instructions (advanced control)
source: InstructionList.pdf
forms: 4
scraped: 2026-06-21
---

# DEF

> **Category:** Basic instructions (advanced control)

## DEF (1)

About DEF user-defined instructions 1) USER (No arguments; no return values) 2) A=USER(X) (With argument; single return value) 3) USER(X) OUT A,B (With argument; multiple return values) Using DEF allows you to define unique instructions as shown above

### DEF

| Item | Description |
| --- | --- |
| `Common Supplement<br>for DEF` | - The definition range should be from DEF to END<br>- Variables and labels defined in the DEF to END range are handled as local<br>- GOTO outside the DEF to END range is impossible<br>- GOSUB or ON GOSUB in the DEF to END range cannot be used<br>They can be used if a SLOT is specified, as in GOSUB "0:@SUB"<br>- They can be used from a different SLOT by adding the COMMON instruction |
| `Specifications for<br>DEF arguments` | - For arguments received with DEF, types will not be checked strictly<br>- Variable names to be received can be specified as necessary by separating them with commas<br>(,)<br>- For string variables, it is also possible to attach $ to the end of a variable name |
| `Specifications for<br>DEF return values` | - For DEF return values, types will not be checked strictly<br>- The type will be determined according to the value written at the beginning of the output<br>variables<br>- When an integer is assigned to a numerical variable, it is handled as integer type<br>- To handle a value as real type, it should be written as in A=100.0<br>- If the type of a return value from DEF is different from the type expected by the recipient,<br>an error will occur |

## DEF (2)

Defines a user instruction with no return values and no arguments

### Format

```sb3
DEF definition name
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

None

### Examples

```sb3
'--- Text display
DEF FUNC
PRINT "SAMPLE"
END
'--- Call
FUNC
```

## DEF (3)

Defines a user function with a single return value

### Format

```sb3
DEF Function name([Argument [,Argument …
])
```

### Arguments

| Argument | Description |
| --- | --- |
| `Specify variable names as necessary if there is an argument or arguments to be passed to the function` |  |

### Return Values

Value to return as a result should be specified after the RETURN instruction

- Notation such as RETURN ANS

### Examples

```sb3
'---Addition
DEF ADD(X,Y)
RETURN X+Y
END
'--- Factorial calculation using recursion
DEF FACTORIAL(N)
IF N==1 THEN RETURN N
RETURN N*FACTORIAL(N-1)
END
'--- Character string inversion
DEF REVERSE$(T$)
VAR A$="" 'Local character string
VAR L=LEN(T$) 'Local
WHILE L>0
 A$=A$+MID$(T$,L-1,1)
 DEC L
WEND
RETURN A$
END
'--- Call
PRINT ADD(10,5)
PRINT FACTORIAL(4)
PRINT REVERSE$("BASIC")
```

## DEF (4)

Defines a user instruction with multiple return values

### Format

```sb3
DEF Instruction name [Argument [,Argument …
]] [OUT V1 [,V2
…
]]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Specify variable names as necessary if there is an argument or arguments to be passed to the function` |  |

### Return Values

Variable names to be returned as a result should be specified as necessary after OUT

### Examples

```sb3
'--- Addition and multiplication
DEF CALCPM A,B OUT OP,OM
OP=A+B
OM=A*B
END
'--- Call
CALCPM 5,10 OUT P,M
PRINT P,M
```
