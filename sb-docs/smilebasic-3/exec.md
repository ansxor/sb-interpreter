---
title: EXEC
slug: docs-sb3-exec
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# EXEC

> **Category:** Files

## EXEC (1)

Loads and executes a program

- It is impossible to return from a program started with EXEC to the previous program
- It is possible to return by using END in a program started with EXEC in another SLOT
- This cannot be used to run a program in DIRECT mode

### Format

```sb3
EXEC "[Resource name:]File name"
```

### Arguments

| Argument | Description |
| --- | --- |
| `Resource name:` | PRG0-PRG3: Program SLOT into which to load the program |
| `File name` | File name of the program to load |

### Examples

```sb3
EXEC "SAMPLE"
EXEC "PRG0:SBGED"
```

## EXEC (2)

Executes a program in a different SLOT

- It is impossible to return from a program executed with EXEC to the previous program
- It is possible to return by using END in a program started with EXEC in another SLOT
- This cannot be used to run a program in DIRECT mode

### Format

```sb3
EXEC Program SLOT
```

### Arguments

| Argument | Description |
| --- | --- |
| `Program SLOT` | 0-3: SLOT number of the program to execute |

### Examples

```sb3
EXEC 2
```
