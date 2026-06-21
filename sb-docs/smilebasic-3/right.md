---
title: RIGHT$
slug: docs-sb3-right
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# RIGHT$

> **Category:** Operations on strings

Extracts a character string with the specified number of characters from the right end of the specified character string

## Format

```sb3
Variable$ = RIGHT$( "Character string", Number of characters )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character string` | Source character string |
| `Number of<br>characters` | Number of characters to extract |

## Return Values

Character string extracted

## Examples

```sb3
S$=RIGHT$("ABC",2)
```
