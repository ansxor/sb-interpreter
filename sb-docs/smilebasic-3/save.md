---
title: SAVE
slug: docs-sb3-save
system: SmileBASIC 3
type: command
category: Files
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SAVE

> **Category:** Files

## SAVE (1)

Saves a file

- When run, a confirmation dialog will be displayed
- The confirmation dialog for SAVE cannot be hidden

### Format

```sb3
SAVE "[Resource name:]File name"
```

### Arguments

| Argument | Description |
| --- | --- |
| `Resource name:` | If omitted: Current program SLOT<br>PRG0-PRG3: Program SLOT (PRG = PRG0)<br>GRP0-GRP5: Graphic page<br>GRPF: Font image page |
| `File name` | Name to save the file under |

### Examples

```sb3
SAVE "PRG0:TEST"
```

## SAVE (2)

Saves a string variable to a text file

### Format

```sb3
SAVE "TXT:File name", String variable
```

### Arguments

| Argument | Description |
| --- | --- |
| `File name` | Name to save the file under (prefixed with "TXT:") |
| `String variable` | String variable containing the text data to be saved (UTF-8) |

### Examples

```sb3
SAVE "TXT:MEMOFILE",TX$
```

## SAVE (3)

Saves a numerical value array to a binary file

### Format

```sb3
SAVE "DAT:File name", Numerical value array
```

### Arguments

| Argument | Description |
| --- | --- |
| `File name` | Name to save the file under (prefixed with "DAT:") |
| `Numerical value<br>array` | Numerical value array containing the data to be saved |

### Examples

```sb3
SAVE "DAT:TEST",MARRAY
```
