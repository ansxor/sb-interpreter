---
title: CHR$
slug: docs-sb3-chr
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# CHR$

> **Category:** Operations on strings

Returns the character for the specified character code

## Format

```sb3
String variable = CHR$( Character code )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Character code` | Number (UTF-16) that corresponds to a character |

## Return Values

Character that corresponds to the character code

## Examples

```sb3
S$=CHR$(65)
```
