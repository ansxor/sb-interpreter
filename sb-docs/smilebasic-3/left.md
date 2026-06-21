---
title: LEFT$
slug: docs-sb3-left
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# LEFT$

> **Category:** Operations on strings

Extracts a character string with the specified number of characters from the left end of the specified character string

## Format

```sb3
String variable = LEFT$( "Character string", Number of characters )
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
S$=LEFT$("ABC",2)
```
