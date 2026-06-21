---
title: STR$
slug: docs-sb3-str
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# STR$

> **Category:** Operations on strings

Gets a character string from a numerical value

## Format

```sb3
String variable = STR$( Numerical value [,Number of digits] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Numerical value to convert to a character string |
| `Number of digits` | - Should be specified when right-justification with a certain number of digits is desired<br>- When the number of digits in the numerical value is greater than the specified number of<br>digits, the specification will be ignored |

## Return Values

| Return Value | Description |
| --- | --- |
| `Character string generated from the numerical value (123` | →<br>"123") |

## Examples

```sb3
S$=STR$( 123 )
```
