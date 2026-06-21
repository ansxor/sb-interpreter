---
title: ASC
slug: docs-sb3-asc
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# ASC

> **Category:** Operations on strings

Gets a character code for the specified character (or string variable)

## Format

```sb3
Variable = ASC( "Character" )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character` | Character string (or string variable) storing the character to check |

## Return Values

```
Character code (UTF-16) for the specified character
```

## Examples

```sb3
A=ASC("A")
```
