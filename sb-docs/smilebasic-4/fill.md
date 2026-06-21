---
title: FILL
slug: docs-sb4-fill
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-fill
content_id: 19570
created: 2023-03-24
scraped: 2026-06-21
---

# FILL

Fill all of part of an array with the same value.

## Syntax

```sb4
FILL array[], value {, start% {, length% }}
```

| Input | Description |
| --- | --- |
| `array[]` | The array to fill. |
| `value` | The value to fill with. |
| `start%` | The index of the first value to fill.<br>Optional, default 0. |
| `length%` | The number of elements to fill.<br>Optional, default `LEN(array[])-start%`. |

The type of `value` must be compatible with the type of `array[]`

`FILL` uses linear indexing for all arrays. There is no built-in function for filling multi-dimensional regions of arrays.

## Examples

```sb4
'fill the entire array
DIM ARRAY%[10]
FILL ARRAY%,100
```

```sb4
'fill a part of an array
DIM ARRAY#[30]
FILL ARRAY#,1.5,5,10
```

```sb4
'2D filling example
'fill from 1,0 to 2,9
DIM ARRAY%[4,10]
FILL ARRAY%,6,10,20
```
