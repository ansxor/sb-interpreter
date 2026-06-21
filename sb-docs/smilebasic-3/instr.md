---
title: INSTR
slug: docs-sb3-instr
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# INSTR

> **Category:** Operations on strings

Searches for the target character string in another character string

## Format

```sb3
Variable = INSTR( [Start position,] "Character string to search in", "Character string to search" )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Start position` | - Position (in character units, larger than or equal to 0) in the source character string from<br>which to start searching<br>- If omitted, the search will be started from the beginning of the source string |
| `Character string<br>to search in` | Source character string |
| `Character string<br>to search for` | Character string to search for in the source character string |

## Return Values

```
- If the search string is found: Position in the source string (in character units)
- Otherwise: -1
```

## Examples

```sb3
A=INSTR( 0, "ABC","B" )
```
