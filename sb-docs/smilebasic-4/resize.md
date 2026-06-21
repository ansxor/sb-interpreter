---
title: RESIZE
slug: docs-sb4-resize
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-resize
content_id: 19571
created: 2023-03-25
scraped: 2026-06-21
---

# RESIZE

Resize the first dimension of an array.

## Syntax

```sb4
RESIZE array[], dim0%, dim1%, dim2%, dim3%
```

| Input | Description |
| --- | --- |
| `array[]` | The array to resize. |
| `dim0%` | The *new* size of the first dimension. |
| `dim1%` | The *current* size of the other dimensions, if they exist. |
| `dim2%` | The *current* size of the other dimensions, if they exist. |
| `dim3%` | The *current* size of the other dimensions, if they exist. |

`RESIZE` can only change the size of the first dimension of `array[]`. It cannot resize any other dimension, or change the number of dimensions. If `array[]` has more than one dimension, the sizes of all dimensions must be specified, even though they cannot be changed.

## Examples

```sb4
DIM ARRAY1[10]
RESIZE ARRAY1,5
PRINT DIM(ARRAY1,0)
```

```sb4
DIM ARRAY2[10,2]
RESIZE ARRAY2,5,2
PRINT DIM(ARRAY2,0)
```

```sb4
DIM ARRAY3[10,2,3]
RESIZE ARRAY3,5,2,3
PRINT DIM(ARRAY3,0)
```

```sb4
DIM ARRAY4[10,2,3,4]
RESIZE ARRAY4,5,2,3,4
PRINT DIM(ARRAY4,0)
```

```sb4
'Error: wrong number of arguments
DIM ARRAY2[10,2]
RESIZE ARRAY2,5
PRINT DIM(ARRAY2,0)
```

```sb4
'Error: trying to change second dimension
DIM ARRAY2[10,2]
RESIZE ARRAY2,10,5
PRINT DIM(ARRAY2,0)
```
