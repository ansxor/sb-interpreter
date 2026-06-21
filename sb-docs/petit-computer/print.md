---
title: PRINT
slug: docs-ptc-print
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-print
content_id: 19559
created: 2023-03-16
scraped: 2026-06-21
---

# PRINT

Print a string to the screen.

## Syntax

```sbsyntax
PRINT {number, ...}
PRINT {string$, ...}
PRINT {number; ...}
PRINT {string$; ...}
```

| Input | Description |
| --- | --- |
| `number` | A numeric variable or literal |
| `string$` | A string variable or literal |

All arguments are optional; you can specify as many or as few as you want, including no argument, which just prints a newline. `PRINT` has no limit on the number of arguments beyond the maximum line length. `?` can also be used in place of `PRINT` - all behavior is the same.

You can print multiple values with one `PRINT` statement using the comma or semicolon. The comma will insert `TABSTEP` spaces before the next argument, whereas the semicolon instead keeps the current position to start printing the next value.

By default, `PRINT` will print a newline after whatever values you print. This can be prevented by adding a comma or semicolon with no following argument. If adding a newline would cause the cursor to go off the screen, the console is instead scrolled.

## Examples

```sb
'prints the number 5
PRINT 5
```

```sb
'prints the string "Hello!"
PRINT "Hello!"
```

```sb
'prints the number 5 followed by a tab, then 6
X=5
Y=6
PRINT X,Y
```

```sb
'prints the string "Score:" immediately followed by 800
SCORE=800
PRINT "Score:";SCORE
```

## Notes

`PRINT` is a very flexible command. It can be modified by `COLOR`, `LOCATE`, and `TABSTEP`. The current location of the text cursor can be read using the `CSRX` and `CSRY` system variables.

One possibly-unintended feature of `PRINT` is that the semicolon can actually be omitted if printing text with no space between.

```sb
' the following is valid code
V=5
T$="WORLD"
PRINT V"HELLO"876T$
```

`PRINT` cannot print an entire array. Attempting to do so prints the value of the variable of the same name.

`PRINT` does not allow instructions to follow it on the same line without a `:`. Attempting to do so causes a `Syntax error`.

## Errors

| Action | Error |
| --- | --- |
| Providing a command directly after PRINT without a ":" or newline (including comments via ') | Syntax error* |
| Providing an invalid expression | Syntax error* |

\*While these actions will cause an error, the `PRINT` statement before or previous valid expressions will still be printed. For example,

```sb
' prints 123456, then causes an error
PRINT 123 456 '789
```

Note that the valid arguments are still printed before the error is thrown. This is unlike many other commands that fail entirely when this happens - one other exception is `LOCATE`.

## See Also

- Console overview
- `LOCATE`
- `CLS`
- `CHKCHR`
