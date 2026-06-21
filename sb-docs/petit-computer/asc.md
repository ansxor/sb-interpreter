---
title: ASC
slug: docs-ptc-asc
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-asc
content_id: 19766
created: 2025-08-18
scraped: 2026-06-21
---

# ASC

Returns the character code from a string.

## Syntax

```sbsyntax
code = ASC(string)
```

| Input | Description |
| --- | --- |
| `string` | String to get character code from |

| Output | Description |
| --- | --- |
| `code` | Character code of first character |

Returns the character code corresponding to the first character of the input string. This code roughly corresponds to ASCII for the English alphanumeric and punctuation characters.

## Examples

```sb
PRINT ASC("A")  ' Prints 65
```

```sb
PRINT ASC("!")  ' Prints 33
```

## Notes

`ASC` only reads the first character if the string. Further characters are ignored.

```sb
PRINT ASC("hello!")  ' Prints 104
PRINT ASC("h")  ' Also prints 104
```

## Errors

| Action | Error |
| --- | --- |
| Less than one argument is specified | Syntax error |
| More than one argument is specified | Missing operand |
| A number is passed for `string` | Type Mismatch |
| An empty string is passed for `string` | Syntax error |

## See Also

- [Function overview](https://smilebasicsource.com/forum/thread/docs-ptc-function)
- [`CHR$`](https://smilebasicsource.com/forum/thread/docs-ptc-chr)
