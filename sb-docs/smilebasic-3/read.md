---
title: READ
slug: docs-sb3-read
system: SmileBASIC 3
type: command
category: Basic instructions (data operations and others)
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# READ

> **Category:** Basic instructions (data operations and others)

Reads the information enumerated with the DATA instruction into the variables Information should be read in the same type as that enumerated with the DATA instruction

## Format

```sb3
READ Acquisition variable 1 [, Acquisition variable 2 …
]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Acquisition<br>variables` | - Variables to store read information (Multiple variables can be specified)<br>- DATA in and after the line specified with the RESTORE instruction will be acquired<br>- If RESTORE is omitted, acquisition will begin with the first occurrence of DATA |

## Examples

```sb3
READ X,Y,Z,G$
DATA 200,120,0,"JAN"
DATA 210,120,0,"FEB"
```
