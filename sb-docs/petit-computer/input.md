---
title: INPUT
slug: docs-ptc-input
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-input
content_id: 19580
created: 2023-03-26
scraped: 2026-06-21
---

# INPUT

Get input from the user and store it to variable(s).

## Syntax

```sbsyntax
INPUT { guide$; } variable {, variable ...}
```

| Input | Description |
| --- | --- |
| `guide$` | Text string to prompt user with. Optional |

| Output | Description |
| --- | --- |
| `variable` | Variable(s) to store user response. Variables can be of numeric or string types, and you can have as many as you want. |

INPUT takes one line of text input from the user and stores the result into the variable or variables given. If provided, the guide string is printed first, followed by a `?`. If the guide string is not provided, only the `?` is printed. If multiple variables are expected, the user must enter all values on the same line with commas between them.

If the user provides invalid input, this command prints `?Redo from start` followed by the prompt again. Note that INPUT can throw errors as well, if user does things such as enter a number that is too large and causes an `Overflow` error.

## Examples

```sb
' Ask the user for their name (text) and display it
INPUT "Name";NAME$
PRINT "Hello, ";NAME$
```

```sb
' Ask the user for name (text) and age (number), and display it
INPUT "Name, Age";NAME$,AGE
PRINT "Hello ";NAME$
PRINT "You are ";AGE;" years old."
```

```sb
' Ask the user for three numbers with the default prompt "?"
PRINT "Please enter three numbers."
INPUT A,B,C
' Display some combinations of those values
PRINT A+B,B+C,A+C
```

## Notes

`INPUT` has a couple of weird properties regarding user input. If asking for multiple values, numbers can be separated using spaces instead of commas. Additionally, some inputs such as ".." or "-" will cause an `Overflow` error if asking for numeric values.

Due to `INPUT` using commas to separate values, it is not possible for the user to input a comma into a string. If this is necessary, use `LINPUT`. Leading spaces are removed, but spaces can be used beyond the first character. Trailing spaces are not removed.

`LOCATE` can be used to alter where the prompt is printed, but the user's input always starts from the beginning of the line. Additionally, if the user enters bad data, the `?Redo from start` and prompt will be at the start of the next line.

If text was already printed to the console where the user's input would be, this is considered part of the user's input and the user can edit it.

It is possible to specify more variables than it is possible to enter on line.

The cursor used by `INPUT` is actually a sprite, with the graphics stored in SPS1U.

## Errors

| Action | Error |
| --- | --- |
| No variable argument is provided | Syntax error |
| A literal argument is provided other than `guide$` | Syntax error |
| User enters a number too large | Overflow |

## See Also

- [Console overview](https://smilebasicsource.com/forum/thread/docs-ptc-console)
- [`LINPUT`](https://smilebasicsource.com/forum/thread/docs-ptc-linput)
- [`INKEY$()`](https://smilebasicsource.com/forum/thread/docs-ptc-inkey)
