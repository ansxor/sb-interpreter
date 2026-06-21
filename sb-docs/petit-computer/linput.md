---
title: LINPUT
slug: docs-ptc-linput
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-linput
content_id: 19583
created: 2023-03-27
scraped: 2026-06-21
---

# LINPUT

Get a string from the user and store it to a variable.

## Syntax

```sbsyntax
LINPUT { guide$; } variable$
```

| Input | Description |
| --- | --- |
| `guide$` | Text string to prompt user with. Optional. |

| Output | Description |
| --- | --- |
| `variable$` | Variable to store result to. |

`LINPUT` takes one line of input from the user and stores the result to a string variable. If provided, the guide string is printed first. If the guide string is not provided, nothing is printed.

## Examples

```sb
' Ask the user for a message (can include comma
LINPUT "Enter message:";MSG$
PRINT "Message: ";MSG$
```

```sb
' Ask the user for text with no prompt
LINPUT TEXT$
```

## Notes

`LINPUT` always stores to exactly one string variable. It is not possible to get multiple variables or numeric variables with `LINPUT`. If you need these, use `INPUT` instead.

`LINPUT` takes all input from the user, including leading and trailing spaces.

`LOCATE` can be used to alter the location of the prompt, or the user's cursor if the guide string is not provided. However, if the user types anything, it will treat the characters before the user's input as spaces, and the user can delete those spaces and move the cursor back. If the user doesn't type any characters and submits, the string submitted will be empty, even if the cursor is past several spaces.

If text was already printed to the console where the user's input would be, this is considered part of the user's input and the user can edit it.

The cursor used by `LINPUT` is actually a sprite, with the graphics stored in `SPS1U`.

## Errors

| Action | Error |
| --- | --- |
| Provide no variable or more than one variable | Syntax error* |
| Provide a number for the guide string | Syntax error |
| Provide a numeric variable as the output | Type Mismatch |

\* If you provide more variables, the input is stored into the first variable and then the error occurs.

## See Also

- [Console overview](https://smilebasicsource.com/forum/thread/docs-ptc-console)
- [`INPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-input)
- [`INKEY$()`](https://smilebasicsource.com/forum/thread/docs-ptc-inkey)
