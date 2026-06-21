---
title: FORMAT$
slug: docs-sb3-format
system: SmileBASIC 3
type: command
category: Operations on strings
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# FORMAT$

> **Category:** Operations on strings

Stringizes values by using display formats to shape them

## Format

```sb3
Variable$ = FORMAT$( "Format string", Value , …
 )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Format string<br>(Multiple formats<br>can be enumerated)` | %S: Outputs the content of the string variable<br>%D: Outputs integers in decimal<br>%X: Outputs integers in hexadecimal<br>%F: Outputs real numbers |
| `Supplemental<br>specifications for<br>format strings` | The following supplemental specifications can be used after % to shape output<br>- Specification of the number of digits: A value indicating the number of digits should be<br>specified (%8D, %4X)<br>- Specification of the number of fractional digits: Should be specified as (number of digits<br>in the integer part).(number of digits in the fractional part) (%8.2F)<br>- Space-padding: A space character and the number of digits should be specified (% 4D<br>→<br>0)<br>- Zero-padding: 0 and the number of digits should be specified (%08D<br>→<br>00000000)<br>- Left alignment: A "-" sign and the number of digits should be specified (%-8D)<br>- Displaying the + sign: A "+" sign and the number of digits should be specified (%+8D) |
| `Value` | - Source value to shape<br>- An adequate number of values corresponding to the elements specified in the formats should<br>be enumerated, separated by commas (,) |

## Return Values

Character string generated

## Examples

```sb3
S$=FORMAT$("%06D",A)
```
