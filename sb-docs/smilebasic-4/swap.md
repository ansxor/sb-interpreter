---
title: SWAP
slug: docs-sb4-swap
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-swap
content_id: 19471
created: 2020-11-03
scraped: 2026-06-21
---

# SWAP

Swap the values of two variables.

## Syntax

```sbsyntax
SWAP var1, var2
```

| Input | Description |
| --- | --- |
| `var1`, `var2` | The two variables to swap. |

The values of the two variables are swapped in-place. `var1` now has `var2`'s previous value, and vice versa. The index operator may also be used to refer to elements of arrays or strings.

## Examples

```sb4
'swap two variables
VAR A=1
VAR B=2
SWAP A,B
PRINT A,B  '2   1
```

```sb4
'swap two array elements in place
DIM ARY[]=[1,2,3]
SWAP ARY[0],ARY[2]
INSPECT ARY
```

```sb4
'swap two characters in a string
VAR A$="ABC"
SWAP A$[0],A$[2]
PRINT A$  'CBA
```
