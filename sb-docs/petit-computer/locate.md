---
title: LOCATE
slug: docs-ptc-locate
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-locate
content_id: 19562
created: 2023-03-20
scraped: 2026-06-21
---

# LOCATE

Set the text cursor location.

## Syntax

```sbsyntax
LOCATE x,y
```

| Input | Description |
| --- | --- |
| x | New x-coordinate of cursor |
| y | New y-coordinate of cursor |

`LOCATE` sets the current cursor location on the console, which determines where text will be printed the next time `PRINT` is called. `LOCATE` will also determine the starting location for `INPUT` and `LINPUT`, but the x-coordinate will be lost after the prompt string is first printed - the user input itself will be from the start of the next line.

## Examples

```sb
' Print "Hello" starting from 5,5
LOCATE 5,5
PRINT "Hello"
```

```sb
' Print a "%" at (X,Y)
LOCATE X,Y:?"% 
```

```sb
CLS
LOCATE 8,5
LINPUT "Enter your name:";NAME$
```

## Notes

The current location of the cursor can be read from system variables `CSRX` and `CSRY`.

All arguments are rounded down to the nearest integer.

Attempting to provide values out of the range of the screen does not set the cursor's location - the arguments are ignored and the previous value is kept.

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are provided | Syntax error |
| One argument is provided | Missing Operand |
| Three or more arguments are provided | Syntax error* |
| A string argument is passed | Type Mismatch |

\*Note that while this will create an error, the first two arguments are still used to set the cursor location if valid. This is different from several other commands, which do not have an effect when a similar error is caused (extra input arguments usually error without performing the command).

## See Also

- Console overview
- `PRINT`
