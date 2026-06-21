---
title: ABS
slug: docs-ptc-abs
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-abs
content_id: 19765
created: 2025-08-18
scraped: 2026-06-21
---

# ABS

Calculates the absolute value of a number.

## Syntax

```sbsyntax
positive = ABS(number)
```

| Input | Description |
| --- | --- |
| `number` | Number to make positive |

| Output | Description |
| --- | --- |
| `positive` | Absolute value of number |

Calculates the absolute value of `number`. This converts a negative number into a positive number, and leaves positive numbers and zero unchanged.

## Examples

```sb
PRINT ABS(-5)  ' Prints 5
```

```sb
PRINT ABS(63)  ' Prints 63
```

## Errors

| Action | Error |
| --- | --- |
| Less than one argument is specified | Syntax error |
| More than one argument is specified | Missing operand |
| A string was passed for any argument | Type Mismatch |

## See Also

- [Function overview](https://smilebasicsource.com/forum/thread/docs-ptc-function)
