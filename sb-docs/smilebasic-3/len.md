---
title: LEN
slug: docs-sb3-len
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# LEN

> **Category:** Operations on strings

Gets the number of characters in a character string/Gets the number of elements in an array

## Format

```sb3
>Variable = LEN( "Character string" or Array variable )
```

## Arguments

| Argument | Description |
| --- | --- |
| `For a character<br>string` | Character string, or the name of the string variable, in which to check the number of<br>characters |
| `For an array<br>variable` | Name of the array variable in which to check the number of elements |

## Return Values

```
- For a character string: Number of characters (All characters will be counted as one character)
- For an array variable: Number of elements
```

## Examples

```sb3
A=LEN("ABC123")
```
