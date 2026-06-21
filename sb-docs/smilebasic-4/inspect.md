---
title: INSPECT / ??
slug: docs-sb4-inspect
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-inspect
content_id: 19472
created: 2020-11-03
scraped: 2026-06-21
---

# INSPECT / ??

Print the type and contents of a value.

## Syntax

```sbsyntax
INSPECT value
?? value
```

| Input | Description |
| --- | --- |
| `value` | The value to inspect. |

The value and its type are printed on the console text screen (`#TCONSOLE`.) Arrays are printed with their dimensions, and the full contents are printed, one element per line, in memory order (last index first). Strings are printed with their length before the contents. The output is truncated to the first 256 characters.

`??` can be used as a shorthand way of writing `INSPECT`.

## Examples

```sb4
'print an int or real
INSPECT 1
'INT: 1
```

```sb4
'print a string
INSPECT "ABC"
'STRING: (3)"ABC"
```

```sb4
'print an array
DIM A%[5,2] = [0,1,2,3,4,5,6,7,8,9]
INSPECT A%
'INT[5,2]:
' [0,0]: 0
' [0,1]: 1
' [1,0]: 2
' [1,1]: 3
' [2,0]: 4
' [2,1]: 5
' [3,0]: 6
' [3,1]: 7
' [4,0]: 8
' [4,1]: 9
```
