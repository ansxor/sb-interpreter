---
title: LOAD
slug: docs-sb3-load
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 4
scraped: 2026-06-21
---

# LOAD

> **Category:** Files

## LOAD (1)

Loads a file

- A confirmation dialog will be displayed
- It is impossible to load a program into the same program SLOT as a running program

### Format

```sb3
LOAD "[Resource name:]File name"[,Dialog display flag]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Resource name:` | If omitted: Current program SLOT<br>PRG0-PRG3: Program SLOT (PRG = PRG0)<br>GRP0-GRP5: Graphic page<br>GRPF: Font image page |
| `File name` | Name of file to load |
| `Dialog display<br>flag` | FALSE = Suppresses confirmation dialog |

### Examples

```sb3
LOAD "PROGNAME"
LOAD "GRP0:GRPDATA"
```

## LOAD (2)

Loads a text file into a string variable

### Format

```sb3
LOAD "TXT:File name"[,Dialog display flag] OUT TX$
```

### Arguments

| Argument | Description |
| --- | --- |
| `File name` | Name of text file to load (prefixed with "TXT:") |
| `Dialog display<br>flag` | FALSE = Suppresses confirmation dialog |

### Return Values

| Return Value | Description |
| --- | --- |
| `TX$` | String variable to store the loaded text file |

### Examples

```sb3
LOAD "TXT:MEMOFILE" OUT TX$
```

## LOAD (3)

Loads a text file into a string variable

### Format

```sb3
String variable = LOAD("TXT:File name" [,Dialog display flag])
```

### Arguments

| Argument | Description |
| --- | --- |
| `File name` | Name of text file to load (prefixed with "TXT:") |
| `String variable` | String variable to store the loaded text file |
| `Dialog display<br>flag` | FALSE = Suppresses confirmation dialog |

### Examples

```sb3
TX$=LOAD("TXT:MEMOFILE")
```

## LOAD (4)

Loads a binary file into a numerical value array

### Format

```sb3
LOAD "DAT:File name", Numerical value array[,Dialog display flag]
```

### Arguments

| Argument | Description |
| --- | --- |
| `File name` | Name of binary file to load (prefixed with "DAT:") |
| `Numerical value<br>array` | Numerical value variable to store the loaded binary file |
| `Dialog display<br>flag` | FALSE = Suppresses confirmation dialog |

### Examples

```sb3
DIM MARRAY[100]
LOAD "DAT:MDATA", MARRAY
```
