---
title: FOR ~ TO ~ STEP ~ NEXT
slug: docs-sb4-for
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-for
content_id: 19509
created: 2020-11-04
scraped: 2026-06-21
---

# FOR ~ TO ~ STEP ~ NEXT

A `FOR` loop is used to repeat a block of code over a range of numbers. It works by setting a counter variable to a starting value and incrementing it by a step value until it exceeds the end value.

## Syntax

```sb4
FOR counter=startVal TO endVal { STEP stepVal }
 statements...
NEXT { counter }
```

| Name | Description |
| --- | --- |
| `counter` | This variable is used to store the current counter value. |
| `startVal` | This is the starting value of `counter`. |
| `endVal` | The `FOR` loop exits when `counter` exceeds this value. |
| `stepVal` | If `STEP` is specified, this value is used as the increment for each loop.<br>If omitted, it is 1. |
| `statements...` | The code inside the block is run on each loop. |

## Examples

`FOR` loops have a variety of potential uses, but they are primarily used for iterating over values.

```sb4
'count the numbers 1 to 10
FOR I=1 TO 10
 PRINT I
NEXT
```

By default `STEP` is always 1, regardless of the order of the start and end values. To count backwards, the `STEP` value must be given a negative number.

```sb4
'count from 10 to 1
FOR I=10 TO 1 STEP -1
 PRINT I
NEXT
```

If `STEP` is positive and start is greater than end, or if `STEP` is negative and end is greater than start, the `FOR` loop will be skipped entirely.

```sb4
'this will never happen
FOR I=1 TO 0
 PRINT I
NEXT
```

The other primary use of `FOR` loops is iterating over elements of an array, by using the counter as an index.

```sb4
'fill this array with its index values
DIM ARY[10]
FOR I=0 TO LAST(ARY)
 ARY[I]=I
NEXT
```

Of course, `FOR` can also be used just to repeat a block of code a fixed number of times.

```sb4
FOR I=1 TO 10
 PRINT "You'll see this ten times"
NEXT
```

## Notes

### NEXT Counter Variable

If you want, you can write an identifier after `NEXT`. This doesn't change the function of the loop; the original purpose of this is to write the counter variable after `NEXT`, to keep track of loops. If multiple `FOR` loops are nested within each other or used close by, this can help you to see where each loop starts and ends.

```sb4
FOR I=0 TO 10
 FOR J=0 TO 10
  FOR K=0 TO 10
   'very long code
  NEXT K
 NEXT J
NEXT I
```

This practice originates from classic variants of BASIC, where it was required. However, in SmileBASIC, it is optional. Since you can write any identifier or literal value after `NEXT` in SmileBASIC, you technically have the freedom to use this feature for any sort of note you want, but more descriptive notes are probably better as comments.

### Condition Evaluation

The end and step values are evaluated at the start of each loop, before the exit condition is checked. This can be used to change the end or step values within the loop itself, controlling its iteration.

```sb4
'print powers of 2
'because I is the step value, I will be doubled with each iteration
FOR I=1 TO 256 STEP I
 PRINT I
NEXT
```

If the counter variable somehow comes before the start value, such as by assigning to it in the `FOR` loop itself, the `FOR` loop will not exit. It will keep iterating.

```sb4
FOR I=1 TO 10
 IF I==2 THEN I=-2
 PRINT I
NEXT
```

You probably never want to do this, because your loop will never end properly. This is just to demonstrate that the `FOR` loop only checks if the counter is "after" the end value.

### Floating-point Error

If the `STEP` value contains fractional digits, e.g. 0.5, the number of loops and the precise value of the counter may be unpredictable, due to floating-point error.
