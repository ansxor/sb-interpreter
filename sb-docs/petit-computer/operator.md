---
title: Operator overview
slug: docs-ptc-operator
system: Petit Computer
type: reference
source: https://smilebasicsource.com/forum/thread/docs-ptc-operator
content_id: 19726
created: 2024-03-31
scraped: 2026-06-21
---

# Operator overview

Operators are symbols that perform various arithmetic, bitwise, or comparison operations. These provide the ability to assign variables, do basic math, and compare numbers and strings.

Each operator has a precedence or priority over other operators. The order of operations is determined by this order, so that each expression can only be evaluated one way. For example, `3+5*6` will evaluate to 33, because multiplication happens before addition.

If two operators have the same precedence, then they are evaluated left-to-right. For example, `3*4%5` evaluates to 2, because first `3*4` is calculated to be `12` and then `12%5` is evaluated to be 2.

## Operator Table

The following table lists operators in groups by their order of operation. Within each block, operators are the same priority, and are evaluated from left to right.

| Operator | Example | Effect |
| --- | --- | --- |
| `()` | `(3+5)*6` -> 48 | Used to perform lower-precedence operations before higher-precedence ones. |
| `()` | `A(5)` -> 0 | Used to access an array element. |
| `[]` | `A[5]` -> 0 | Used to access an array element. |
| --- | --- | --- |
| Func. | `ABS(-5)` -> 5 | Calls a function and returns the result. The number of arguments for a function can vary. See [Function overview](https://smilebasicsource.com/forum/thread/docs-ptc-function). |
| --- | --- | --- |
| `-` | `- 5` ->  -5 | Negates a value. |
| `!` | `!TRUE` -> 0 (FALSE) | Logical inversion - converts truthy (nonzero) values to 0 (FALSE) and falsy (zero) values to 1 (TRUE). |
| `NOT` | `NOT &H13579` -> -79226 (&HECA86) | Bitwise inversion - converts each 1 bit to a 0 bit and each 0 bit to a 1 bit. |
| --- | --- | --- |
| `*` | `"A"*5` -> "AAAAA" | Repeats the string some number of times. |
| `*` | `3*5` -> 15 | Multiplies two numbers together. |
| `/` | `3/5` -> 0.6 | Divides the first number by the second. Division by zero will cause an error. |
| `%` | `5%3` -> 2 | Calculates the remainder of the first number divided by the second number. Dividing by zero will cause an error. |
| --- | --- | --- |
| `+` | `5+3` -> 8 | Adds two numbers. |
| `+` | `"A"+"B"` -> "AB"  | Concatenates two strings. |
| `-` | `5-3` -> 2 | Subtracts the second number from the first number. |
| --- | --- | --- |
| `==` | `1==1` -> 1 (TRUE) | Evaluates to TRUE if two numbers are equal, and FALSE if they are not equal. |
| `==` | `"A"=="B"` -> 0 (FALSE) | Evaluates to TRUE if two strings are the same, and FALSE if they are different. |
| `!=` | `1!=1` -> 0 (FALSE) | Evaluates to TRUE if the two numbers are different, and FALSE if they are equal. |
| `!=` | `"A"!="B"` -> 1 (TRUE) | Evaluates to TRUE if the two strings are different, and FALSE if they are equal. |
| `<=` | `3<=3` -> 1 (TRUE) | Evaluates to TRUE if the first is less than or equal to the second, otherwise FALSE. |
| `<=` | `"A"<="B"` -> 1 (TRUE) | Evaluates to TRUE if the first operand precedes the second operand in lexicographical order or is equivalent to the second operand, otherwise FALSE. |
| `>=` | `3>=5` -> 0 (FALSE) | Evaluates to TRUE if the first is greater than or equal to the second, otherwise FALSE. |
| `>=` | `"A">="B"` -> 0 (FALSE) | Evaluates to TRUE if the first operand succeeds the second operand in lexicographical order or is equivalent to the second operand, otherwise FALSE. |
| `<` | `3<5` -> 1 (TRUE) | Evaluates to TRUE if the first number is less than the second number, otherwise FALSE. |
| `<` | `"A"<"B"` -> 1 (TRUE) | Evaluates to TRUE if the first operand precedes the second operand in lexicographical order, otherwise FALSE. |
| `>` | `3>5` -> 0 (FALSE) | Evaluates to TRUE if the first number is greater than the second number, otherwise FALSE. |
| `>` | `"A">"B"` -> 0 (FALSE) | Evaluates to TRUE if the first operand succeeds the second operand in lexicographical order, otherwise FALSE. |
| --- | --- | --- |
| `AND` | `25 AND 42` -> 8 | Calculates a bitwise AND of two integers. Removes fractional component of result. |
| `OR` | `25 OR 42` -> 59 | Calculates a bitwise OR of two integers. Removes fractional component of result. |
| `XOR` | `25 XOR 42` -> 51 | Calculates a bitwise XOR of two integers. Removes fractional component of result. |
| --- | --- | --- |
| `=` | `A=3` | Assigns values to variables. Can assign both numbers and strings. |

## Notes

The ordering of strings is based on the UCS-2 character code of the string. This does not correspond to the codes used by `CHR$` and `ASC`, but instead the internal encoding of Petit Computer. This order matches alphabetical order within the English alphabet, with all lowercase letters sorting after uppercase letters.

## Errors

If the result of a numeric operation exceeds the range of [-524287.999994, 524287.999994], the operation will cause an `Overflow` error.

If the result of a string operation causes the string to exceed the maximum length of 256 characters, the operation will cause a `String too long` error.

If a string operation is performed and the program does not have any remaining string memory available for the result, an `Out of memory` error will occur.
