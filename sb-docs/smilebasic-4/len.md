---
title: LEN
slug: docs-sb4-len
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-len
content_id: 19476
created: 2020-11-01
scraped: 2026-06-21
---

# LEN

Check the length of an array or string.

## Syntax

```sbsyntax
length% = LEN(array[])
length% = LEN(string$)
LEN array[] OUT length%
LEN string$ OUT length%
```

| Input | Description |
| --- | --- |
| `array[]` | The array whose length to check. |
| `string$` | The string whose length to check. |

| Output | Description |
| --- | --- |
| `length%` | The length of the array or string. |

`length%` will be the *total number* of characters in the string or elements in the array. To get the number of dimensions in an array or the length of each dimension, use the `DIM` function.

## Examples

```sb4
'check the length of this string
PRINT LEN("HELLO")  '5
```

```sb4
'check the length of an array
DIM ARY[10]
PRINT LEN(ARY)  '10
```

```sb4
'LEN returns the *total number* of elements
DIM ARY[5,2]
PRINT LEN(ARY)  '5*2 = 10
```
