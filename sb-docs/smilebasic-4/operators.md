---
title: Operator Table
slug: docs-sb4-operators
system: SmileBASIC 4
type: reference
source: https://smilebasicsource.com/forum/thread/docs-sb4-operators
content_id: 19452
created: 2020-11-18
scraped: 2026-06-21
---

# Operator Table

| Syntax | Name | Description | Example |
| --- | --- | --- | --- |
| `()` | Parentheses | Groups sub-expressions (effectively raising their precedence) and surrounds argument lists for function calls. | `1+(2+3)`<br>`RND(10)` |
| `[]` | Index Reference | Refer to an element of an array, or a character of a string, given its index. | `ARRAY[10]` |
| `+` | Addition | Add two numbers. | `1+2` |
| `+` | String Concatenation | Concatenate two strings. | `"abc"+"def"` |
| `-` | Negation | Negate a number. | `-10` |
| `-` | Subtraction | Subtract two numbers. | `5-2` |
| `*` | Multiplication | Multiply two numbers. | `5*2` |
| `*` | String Repetition | Repeat the contents of a string a given number of times. | `"abc"*3` |
| `/` | Division | Divide two numbers. Result is a real number. | `1/2` |
| `DIV` | Integer Division | Truncating integer division of two numbers. Result is an integer. | `1 DIV 2` |
| `MOD` | Integer Modulo | Remainder of truncating integer division. Result is an integer. | `1 MOD 2` |
| `<<` | Left Bit Shift | Left shift an integer by /n/ bits. | `1<<16` |
| `<<<` | Left Bit Shift | Left shift an integer by /n/ bits. | `1<<<16` |
| `<<+` | Left Bit Rotate | Left rotate an integer by /n/ bits. The highest bit is rotated into the lowest bit. | `1<<+16` |
| `>>` | Signed Right Bit Shift | Right shift an integer by /n/ bits. The sign bit is filled with its previous value (sign extension.) | `-500>>2` |
| `>>>` | Unsigned Right Bit Shift | Right shift an integer by /n/ bits. The sign bit is filled with zero (zero extension.) | `-500>>>2` |
| `>>+` | Right Bit Rotate | Right rotate an integer by /n/ bits. The lowest bit is rotated into the highest bit. | `1>>+16` |
| `AND` | Bitwise AND | Find the bitwise AND of two integers. | `16 AND 32` |
| `OR` | Bitwise OR | Find the bitwise OR of two integers. | `16 OR 32` |
| `XOR` | Bitwise XOR | Find the bitwise exclusive OR of two integers. | `16 XOR 32` |
| `NOT` | Bitwise NOT | Invert all bits of an integer. | `NOT 10` |
| `==` | Equality | True if the two values are equal. | `1==2` |
| `!=` | Inequality | True if the two values are not equal. | `1!=2` |
| `<` | Less Than | True if the first value is less than the second. | `1<2` |
| `>` | Greater Than | True if the first value is greater than the second. | `1>2` |
| `<=` | Less Than or Equal | True if the first value is less than or equal to the second. | `1<=2` |
| `>=` | Greater Than or Equal | True if the first value is greater than or equal to the second. | `1>=2` |
| `!` | Logical NOT | Inverts the condition (true is false, false is true) | `!(1<2)` |
| `&&` | Logical AND | True if both sides are true. If the left side is false, the right side is not evaluated (short-cutting.) | `1<2&&3<4` |
| `\|\|` | Logical OR | True if either side is true. If the left side is true, the right side is not evaluated (short-cutting.) | `1<2\|\|3<4` |

## Precedence

The order of operations in an expression is determined by the operator precedence. The first row is highest precedence, and the last row is lowest. Operators on the same row have the same precedence, and they're evaluated by left-to-right order of the expression.

| Precedence |
| --- |
| Highest |
| `()` `[]` |
| `NOT` `!` `-` (Negation) |
| `*` `/` `DIV` `MOD` |
| `+` `-` (Subtraction) |
| `<<` `<<<` `<<+` `>>` `>>>` `>>+` |
| `==` `!=` `<` `<=` `>` `>=` |
| `AND` |
| `OR` `XOR` |
| `&&` |
| `\|\|` |
| Lowest |
