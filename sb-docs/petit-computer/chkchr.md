---
title: CHKCHR
slug: docs-ptc-chkchr
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-chkchr
content_id: 19569
created: 2023-03-23
scraped: 2026-06-21
---

# CHKCHR

Read a character from the console.
## Syntax
```sbsyntax
code = CHKCHR(x, y)
```
|Input | Description |
| --- | --- |
|`x`| X coordinate to read from. |
|`y`| Y coordinate to read from. |
|Output | Description |
| --- | --- |
|`code`| Character code read from console. |

Reads the character located at (x,y) and returns a character code in range of 0-255 from the console. If attempting to read from off the edge of the screen, the return value is -1 instead.

## Examples
```sb
' Set up characters to read
CLS
PRINT "ABC"
' will print 65
?CHKCHR(0,0)
' will print 67
?CHKCHR(2,0)
' will print 0
?CHKCHR(5,5)
' will print -1
?CHKCHR(-1,-1)
```

## Notes
All arguments are rounded down.

When the console has been cleared with `CLS`, the character code read from anywhere on screen will be 0.

`CHKCHR` does not correctly read characters that aren't in the usual character set - if characters outside the normal range are printed, they will still return codes in the 0-255 range, but offset by 32 (0x20). For example, if using `CHKCHR` to read character 256 (character 0 of second character bank), the result would be 32.

`CHKCHR` has a strange bug in which exactly one string argument can be passed and treated as a number. The empty string is treated as -1; every other string is treated as 0. 
```sb
' Equivalent to CHKCHR(0,-1)
?CHKCHR(0,"")
' Causes an error
?CHKCHR("","")
```

There is no `CHKCHR` equivalent for the lower screen console.

## Errors
|Action | Error |
| --- | --- |
| One or zero arguments are passed | Syntax error |
| Three or more arguments are passed | Missing operand |
| Two string arguments are passed | Type Mismatch |

## See Also
- Console overview
- `PRINT`
- `CHR$`
- `ASC`
