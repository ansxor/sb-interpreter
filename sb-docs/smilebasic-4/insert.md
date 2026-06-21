---
title: INSERT
slug: docs-sb4-insert
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-insert
content_id: 19572
created: 2023-03-25
scraped: 2026-06-21
---

# INSERT

Insert values into a 1D array.

## Syntax

```sb4
INSERT array[], index% {, amount% {, value }}
```

| Input | Description |
| --- | --- |
| `array[]` | The array to insert values into. |
| `index%` | The index to insert values at. |
| `amount%` | The number of values to insert.<br>Optional, default 1. |
| `value` | The value to insert. Optional |

`INSERT` resizes `array[]` by inserting elements at `index%`. Elements after the insertion index are shifted toward the end. By default, one element is inserted and assigned to the default value of the array's type.

| Type | Value |
| --- | --- |
| Integer | `0` |
| Real | `0.0` |
| String | `""` |

`index%` cannot be less than 0 or more than the length of `array[]`. If `index%` equals the length, the new elements are added at the end.

If `amount%` is passed, then multiple new elements are added starting at `index%`; e.g. if `index%` is 4 and `amount%` is 3, then new elements are added at 4, 5, and 6.

If `value` is passed, then that value is assigned to all new indices instead of the default. `value` and `array[]` must have compatible types.

`INSERT` cannot be used on multidimensional arrays.

## Examples

```sb4
'insert zero
DIM ARRAY[2]=[1,2]
INSERT ARRAY,1
INSPECT ARRAY
```

```sb4
'insert three zeros
DIM ARRAY[2]=[1,2]
INSERT ARRAY,1,3
INSPECT ARRAY
```

```sb4
'insert three tens
DIM ARRAY[2]=[1,2]
INSERT ARRAY,1,3,10
INSPECT ARRAY
```

## See Also

- [Array Guide](https://smilebasicsource.com/forum/thread/docs-sb4-array-guide)
- [`RESIZE`](https://smilebasicsource.com/forum/thread/docs-sb4-resize)
- [`REMOVE`](https://smilebasicsource.com/forum/thread/docs-sb4-remove)
- [`PUSH`](https://smilebasicsource.com/forum/thread/docs-sb4-push)
- [`POP`](https://smilebasicsource.com/forum/thread/docs-sb4-pop)
- [`UNSHIFT`](https://smilebasicsource.com/forum/thread/docs-sb4-unshift)
- [`SHIFT`](https://smilebasicsource.com/forum/thread/docs-sb4-shift)
