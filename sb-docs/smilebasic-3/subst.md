---
title: SUBST$
slug: docs-sb3-subst
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SUBST$

> **Category:** Operations on strings

Substitutes one character string with another string

## Format

```sb3
String variable = SUBST$( "Character string", Start position, [Number of characters,] "Substitute string" )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Source character string |
| `Start position` | Position in the source character string from which to start substitution (0 - Number of<br>characters minus 1) |
| `Number of<br>characters` | - Number of characters to substitute with another string<br>- If omitted, all characters after the substitution start position will be replaced with the<br>substitute string |
| `Substitute string` | The specified number of characters from the start position will be substituted with this<br>string |

## Return Values

Character string after the substitution

## Examples

```sb3
A$=SUBST$( "ABC",0,2,"XY" )
```
