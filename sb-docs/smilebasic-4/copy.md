---
title: COPY
slug: docs-sb4-copy
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-copy
content_id: 19478
created: 2020-11-11
scraped: 2026-06-21
---

# COPY

Copy elements to and from arrays or strings. The function comes broadly in three forms: one that copies elements into a new array/string, one that copies elements from a source into a destination, and one that copies values from `DATA` statements into an existing array.

## Copy to New

```sbsyntax
new[] = COPY(source[] {, offset% {, amount% }})
new$ = COPY(source$ {, offset% {, amount% }}
COPY source[] {, offset% {, amount% }} OUT new[]
COPY source$ {, offset% {, amount% }} OUT new$
```

| Input | Description |
| --- | --- |
| `source[]` | A 1D array to copy values from. |
| `source$` | A string to copy characters from. |
| `offset%` | The index to start copying from. Optional, default 0. |
| `amount%` | The amount of elements to copy. Optional.<br>If not specified, the number of items after `offset%` (or `LEN(source) - offset%`) is used. |

| Output | Description |
| --- | --- |
| `new[]` | A new 1D array of appropriate type, containing the values copied from `source[]`. |
| `new$` | A new string containing the characters copied from `source$`. |

Note that this form of `COPY` only supports 1D arrays. To copy from arrays with more dimensions, you must use the destination-source form of `COPY`.

## Copy to Destination

```sbsyntax
COPY dest[] {, destOffset% }, source[] {{, sourceOffset% }, amount% }
COPY dest$ {, destOffset% }, source$ {{, sourceOffset% }, amount% }
```

| Input | Description |
| --- | --- |
| `dest[]` | The destination array to copy items to. |
| `dest$` | The destination string to copy characters to. |
| `destOffset%` | The starting index in the destination where items will be copied. Optional, default 0. |
| `source[]` | The source array to copy items from. |
| `source$` | The source string to copy characters from. |
| `sourceOffset%` | The starting index in the source to copy items from. Optional, default 0. |
| `amount%` | The amount of items to be copied. Optional.<br>If not specified, the number of items after `sourceOffset%` (or `LEN(source) - sourceOffset%`) is used. |

This form of `COPY` is used to copy items between existing arrays or strings. The contents copied from `source` are written into `dest` in-place starting at `destOffset%`. Copying and indexing is handled as though either array is 1D (specifically, as they are formatted in memory.) If the destination array is 1D and the copy operation runs past its end, then the destination array is resized to fit all copied elements.

## Copy from DATA

```sbsyntax
COPY dest[] {, offset% }, @label {, amount% }
```

| Input | Description |
| --- | --- |
| `dest[]` | The destination array to copy items to. |
| `offset%` | The index in `dest[]` to start copying items to. Optional, default 0. |
| `@label` | A label to begin reading `DATA` from. |
| `amount%` | The amount of `DATA` values to read. Optional.<br>If not specified, the number of items after `offset%` (or `LEN(source) - offset%`) is used. |

Values from `DATA` statements, starting at `@label`, are copied into `dest[]`. Note that there must be enough `DATA` values to copy, or you will get `Out of DATA`, and that all values are compatible with the type of `dest[]`, or you will get `Type mismatch`.

## Examples

```sb4
'copy a subsection of an array
DIM ARY[]=[0,1,2,3,4,5,6,7,8,9]
VAR ARY2=COPY(ARY,2,3)
INSPECT ARY2
```

```sb4
'copy a string on assignment
VAR A$="ABCDE"
VAR B$=COPY(A$)
```

```sb4
'copy items from DATA
@PRIMES
DATA 1,2,3,5,7
DIM P[5]
COPY P,@PRIMES
```
