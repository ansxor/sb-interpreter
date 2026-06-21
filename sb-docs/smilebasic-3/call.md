---
title: CALL
slug: docs-sb3-call
system: SmileBASIC 3
type: command
category: Basic instructions (advanced control)
source: InstructionList.pdf
forms: 4
scraped: 2026-06-21
---

# CALL

> **Category:** Basic instructions (advanced control)

## CALL (1)

Calls the user-defined instruction with the specified name

### Format

```sb3
CALL "Instruction name" [,Argument …
] [OUT Variable 1 [,Variable 2
…
]]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Instruction name` | - User-defined instruction name string to call<br>- Being a string, it should either be enclosed in "" or specified using a string variable. |
| `Arguments-` | Any arguments required for the specified instruction |

### Return Values

Variable names to return as a result should be specified as necessary after OUT

### Examples

```sb3
CALL "USERCD",X,Y OUT A,B
'
DEF USERCD X,Y OUT A,B
A=X+Y:B=X*Y
END
```

## CALL (2)

Calls the user-defined function with the specified name

### Format

```sb3
Variable=CALL("Function name" [,Argument …
])
```

### Arguments

| Argument | Description |
| --- | --- |
| `Function name` | - User-defined function name string to call<br>- Being a string, it should either be enclosed in "" or specified using a string variable. |
| `Arguments` | Any arguments required for the specified function should be enumerated |

### Examples

```sb3
A=CALL("USERFC",X,Y)
'
DEF USERFC(X,Y)
RETURN X*Y
END
```

## CALL (3)

Calls a sprite callback Processes which have been specified for each sprite using SPFUNC are called together

### Format

```sb3
CALL SPRITE
```

### Examples

```sb3
CALL SPRITE
```

## CALL (4)

Calls a BG callback Processes which have been specified for each sprite using SPFUNC are called together

### Format

```sb3
CALL BG
```

### Examples

```sb3
CALL BG
```
