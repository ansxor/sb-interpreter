---
title: PRINT
slug: docs-sb4-print
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-print
content_id: 19651
created: 2023-06-07
scraped: 2026-06-21
---

# PRINT

Print text to the console.

## Syntax

```sb4
PRINT { expr, ... }
PRINT { expr; ... }
?{ expr, ... }
?{ expr; ... }
```

| Input  | Description |
| --- | --- |
| `expr` | Any expression of any type (variable, math etc.) |

`PRINT` will print the result of any expression/value as text to the console text screen (4, `#TCONSOLE`.) Any number of arguments can be passed (even none.) This keyword is equivalent to [`TPRINT`](https://smilebasicsource.com/forum/thread/docs-sb4-tprint), but with no text screen ID argument. `?` can also be used instead of `PRINT`.

## Expressions

Any value or expression that is not an array or empty value can be printed. Strings are printed as-is (there is no support for formatting codes.) Integers and real numbers are printed in the same format as `STR$`.

## Cursor/Printing Position

Text is printed starting at the current console cursor position. This is set by `LOCATE` or advanced automatically by a `PRINT`/`TPRINT`. Text that passes the right edge of the console is wrapped to the next line. If text is printed over existing text on the console, the old text is overwritten.

By default, `PRINT` advances the cursor to the next line after printing.

## Commas and Semicolons

Arguments can be separated by either a comma or a semicolon. If a comma is written after an argument, then the cursor is advanced to the next `TABSTEP` column on the same line, or to the next line if the cursor would pass the edge. This allows values to be grouped into columns when printed. The default of `TABSTEP` is 4.

```sb4
PRINT 1,2,3,4
'1   2   3   4
```

In this example, each printed value is one character and `TABSTEP` is 4. The starting cursor position is 0, so 1 is printed at 0, 2 is printed at 4, 3 is printed at 8, etc. The cursor is advanced by printing enough spaces to align to `TABSTEP`.

If a semicolon is written after an argument, no spaces are inserted after it. The cursor is advanced to right after the argument (or to the next line if at the edge.)

```sb4
PRINT 1;2;3;4
'1234
```

Semicolons and commas can be mixed.

```sb4
PRINT "Position: ";X,Y
'Position: 10    20
```

If the argument list ends with a comma or semicolon, a new line isn't printed. Instead, the cursor is advanced (or not) to its next position.

```sb4
PRINT 1
PRINT 2
PRINT 3
'1
'2
'3
```

```sb4
PRINT 1,
PRINT 2,
PRINT 3,
'1   2   3
```

```sb4
PRINT 1;
PRINT 2;
PRINT 3;
'123
```
