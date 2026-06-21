---
title: DIM()
slug: docs-sb4-dim
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-dim
content_id: 19480
created: 2020-11-12
scraped: 2026-06-21
---

# DIM()

Check the dimensions of an array.

> *Note:* Do not confuse the `DIM()` function with the `DIM` statement for declaring variables.

## Check Number of Dimensions

```sbsyntax
dimensions% = DIM(array[])
```

| Input | Description |
| --- | --- |
| `array[]` | The array to check. |

| Output | Description |
| --- | --- |
| `dimensions%` | The number of dimensions `array[]` has. |

`dimensions%` will be 1 for a 1D array, 2 for a 2D, etc.

## Check Length of a Dimension

```sbsyntax
length% = DIM(array[], dimension%)
```

| Input | Description |
| --- | --- |
| `array[]` | The array to check. |
| `dimension%` | The dimension of array to check the length of. |

| Output | Description |
| --- | --- |
| `length%` | The length of `dimension%` in `array[]`. |

The value `dimension%` corresponds to the "n th dimension," i.e. 0 is the first dimension in `array[]`. `length%` will the size of that specific dimension.

## Examples

```sb4
'check dimensions of A
DIM A[10,2]
PRINT DIM(A)    '2
PRINT DIM(A,0)  '10
```

## Notes

### No OUT Form

It is impossible to call `DIM()` in `OUT` form. Though it appears to be a proper "function", it is a special case of the `DIM` keyword that behaves like one. Take the following example:

```sb4
DIM A[]
VAR B
DIM A OUT B
```

When the parser reaches line 3, it tries to parse it as a variable declaration, because it sees the `DIM` keyword and then an identifier. If `A` was declared earlier (in this example it was, and in most cases it would be) this line will trigger `Duplicate variable`, because this is treated as a `DIM` statement. The parser doesn't even have a chance to reach the `OUT` keyword. If `A` was not declared previously (maybe you aren't using strict mode) this line will still throw a `Syntax error`, because using `OUT` after a variable declaration is meaningless. So, in all cases, this must be called like a single-return function (which, in most cases, is what you would do anyway.)

```sb4
DIM A[]
VAR B=DIM(A)
```

Perhaps this also explains the lack of an `OUT` form that returns the lengths of all dimensions, e.g. `DIM A OUT B,C,D,E`.
