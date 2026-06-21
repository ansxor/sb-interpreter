---
title: ARYOP
slug: docs-sb4-aryop
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-aryop
content_id: 19496
created: 2020-05-06
scraped: 2026-06-21
---

# ARYOP

Perform a batch operation on a real number array, using additional parameters.

## Syntax

```sbsyntax
ARYOP operation%, output#[], param1#, param2# {, param3# }
```

| Input | Description |
| --- | --- |
| `operation%` | The operation to perform. Can be one of the following:<br><br>\| Constant \| Value \| Description \|<br>\| --- \| --- \| --- \|<br>\| `#AOPADD` \| 0 \| Add: `p1+p2` \|<br>\| `#AOPSUB` \| 1 \| Subtract: `p1-p2` \|<br>\| `#AOPMUL` \| 2 \| Multiply: `p1*p2` \|<br>\| `#AOPDIV` \| 3 \| Divide: `p1/p2` \|<br>\| `#AOPMAD` \| 4 \| Multiply-add: `p1*p2+p3` \|<br>\| `#AOPLIP` \| 5 \| Linear interpolate: `p1*p3+p2*(1-p3)` \|<br>\| `#AOPCLP` \| 6 \| Clamp: `p2<=p1<=p3` \| |
| `output#[]` | Array used to store results. |
| `param1#` | First parameter to the operation. |
| `param2#` | Second parameter to the operation. |
| `param3#` | Third parameter to the operation. Used only for operations with three parameters. |

The operation is computed using the parameters and stored in each element of the output array. Refer to the formulas in the operation table to see how each operation is computed.

Each parameter can be either a number or an array of numbers. When the parameter is a number, the same number is used for every step. If the parameter is an array, the corresponding element at each step is used in the operation. If the length of a parameter array is less than the length of the output array, the array is repeated so that there is enough elements.

## Examples

```sb4
'sample array data
DIM A#[3] = [1,2,3]
DIM B#[3] = [4,5,6]
DIM C#[3]

'find the sum of two arrays
ARYOP #AOPADD,C#,A#,B#
??C#  '> [5,7,9]

'multiply an entire array by 2
ARYOP #AOPMUL,C#,A#,2
??C#  '> [2,4,6]

'clamp all numbers between 3 and 5
ARYOP #AOPCLP,C#,C#,3,5
??C#  '> [3,4,5]
```
