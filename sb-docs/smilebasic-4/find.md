---
title: FIND
slug: docs-sb4-find
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-find
content_id: 19479
created: 2020-11-11
scraped: 2026-06-21
---

# FIND

Find the index of a value within an array.

## Syntax

```sbsyntax
FIND array[], value OUT index0% {, index1% {, index2% {, index3% }}}
index0% = FIND(array[], value)
```

| Input | Description |
| --- | --- |
| `array[]` | The array to search. |
| `value` | The value to find. |

| Output | Description |
| --- | --- |
| `index0%` | The index (or indices) of the first occurrence of `value`.<br>If `value` is not found, -1. |
| `index1%` | The index (or indices) of the first occurrence of `value`.<br>If `value` is not found, -1. |
| `index2%` | The index (or indices) of the first occurrence of `value`.<br>If `value` is not found, -1. |
| `index3%` | The index (or indices) of the first occurrence of `value`.<br>If `value` is not found, -1. |

The number of return values must correspond to the number of dimensions in `array[]`; `index0%` corresponds to the first index, `index1%` the second, etc. Altogether these values tell you the location of the first occurrence of `value` within `array[]`. If `value` is not found, then all index values will be -1.

Note that, since all arrays can be indexed as though they are 1D, all arrays can be used with the single-return form of `FIND`. The returned index (`index0%` in this case) will simply be the 1D index of `value`. Also note that this function can only ever find the *first* occurrence of `value` because you cannot specify a starting index.

## Examples

```sb4
DIM ARY[]=[3,6,1,4,2,5,7,0,9,8]
PRINT FIND(ARY,4)  '3
```

```sb4
DIM ARY[3,3]=\
 [1,2,3,\
  4,5,6,\
  7,8,9]
FIND 4 OUT Y,X
PRINT X,Y  '0 1
PRINT FIND(4)  '3
```
