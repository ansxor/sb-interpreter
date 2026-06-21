---
title: HEX$
slug: docs-sb3-hex
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# HEX$

> **Category:** Operations on strings

Gets a hexadecimal string from a numerical value

## Format

```sb3
String variable = HEX$( Numerical value [,Number of digits] )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Numerical value from which to get a hexadecimal string (The fractional part should be<br>truncated) |
| `Number of digits` | - Number of digits in the hexadecimal string to output<br>- If specified, the string will be padded with leading zeros before being returned |

## Return Values

| Return Value | Description |
| --- | --- |
| `Hexadecimal string generated from the numerical value (255` | →<br>"FF") |

## Examples

```sb3
S$=HEX$(65535,4)
```
