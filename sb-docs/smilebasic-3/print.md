---
title: PRINT
slug: docs-sb3-print
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# PRINT

> **Category:** Console input/output

Displays characters on the console screen

- Omitting expressions causes only a line break to occur
- ? can be used instead of PRINT

## Format

```sb3
PRINT [Expression [; or, Expression …
]]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Expression` | - Variables, string variables, numerical values, or character strings to display<br>- Formulas are also allowed, including the four arithmetic operations, and functional<br>calculations (The calculation results will be displayed) |
| `; (semicolon)` | Without beginning a new line after the previous display item, displays the next display item<br>without any space |
| `, (comma)` | - Without beginning a new line after the previous display item, places a set interval before<br>the next display item<br>- The display location is determined according to a system variable (the TABSTEP unit) |

## Examples

```sb3
PRINT "RESULT(X,Y)=";DX*4+1,DY+1
```
