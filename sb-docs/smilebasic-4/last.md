---
title: LAST
slug: docs-sb4-last
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-last
content_id: 19477
created: 2020-11-01
scraped: 2026-06-21
---

# LAST

Get the index of the last element in an array or string.

## Syntax

```sbsyntax
lastIndex% = LAST(array[])
lastIndex% = LAST(string$)
LAST array[] OUT lastIndex%
LAST string$ OUT lastIndex%
```

| Input | Description |
| --- | --- |
| `array[]` | The array whose last index to get. |
| `string$` | The string whose last index to get. |

| Output | Description |
| --- | --- |
| `lastIndex%` | The last index of the array or string. |

`lastIndex%` is the index of the last character in `string$` or the last element of `array[]`. All arrays are treated as linear; `LAST(v)` is always equivalent to `LEN(v)-1`.

## Examples

```sb4
'get the last index of this string
PRINT LAST("HELLO")  '4
```

```sb4
'get the last index of an array
DIM ARY[10]
PRINT LAST(ARY)  '10
```

```sb4
'all arrays are treated linearly
DIM ARY[5,2]
PRINT LAST(ARY)  '5*2-1 = 9
```
