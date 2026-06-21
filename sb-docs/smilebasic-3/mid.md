---
title: MID$
slug: docs-sb3-mid
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# MID$

> **Category:** Operations on strings

Extracts a character string with the specified number of characters from the specified position in the specified character string

## Format

```sb3
String variable = MID$( "Character string", Start position, Number of characters )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Source character string |
| `Start position` | Position (in character units) from which to start extracting a character string |
| `Number of<br>characters` | Number of characters to extract |

## Return Values

Character string extracted

## Examples

```sb3
S$=MID$("ABC",0,2)
```
