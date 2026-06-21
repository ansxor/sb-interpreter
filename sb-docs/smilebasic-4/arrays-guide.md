---
title: Arrays Guide
slug: docs-sb4-arrays-guide
system: SmileBASIC 4
type: guide
source: https://smilebasicsource.com/forum/thread/docs-sb4-arrays-guide
content_id: 19481
created: 2022-01-01
scraped: 2026-06-21
---

# Arrays Guide

An *array* is a data type that contains multiple values. They can contain only values of the same type, and can have multiple dimensions.

## Basics

An array allows you to store multiple values in one data structure. Each value, or *element*, is assigned to an *index*.

```sb4
DIM ARRAY%[3]
ARRAY%[0]=1
ARRAY%[1]=2
ARRAY%[2]=3
PRINT ARRAY%[0]
```

Arrays can be treated like any other value in many situations. There are functions to work specifically with arrays, as well.

```sb4
INSPECT ARRAY%
PRINT LEN(ARRAY%)
FILL ARRAY%,100
```

An array can contain only values of one type.

```sb4
DIM FOO%[1]  'integers
DIM FOO#[1]  'reals
DIM FOO$[1]  'strings
FOO%[0] = "a string"  'not allowed!
```

Because arrays are typed and variables are not, this can sometimes become confusing. In this page, all array variables will be named with type suffixes. We recommend you also do this.

## Creation

Arrays are created one of two ways: as part of variable initialization with `DIM`, or using one of the `ARRAY` functions to return a new array.

### Declaration and Initialization

The `DIM` keyword can be used to declare and initialize variables as arrays.

```sb4
DIM array[length]
```

`array` is the variable name, and `length` is the number of elements. Note that `DIM` is the same as a variable declaration. To reinitialize a variable with an array, you cannot use `DIM` twice.

```sb4
DIM FOO%[3]
' no!
DIM FOO%[2]
```

Array declarations can also contain an *initializer*. This is a list of values surrounded by square brackets.

```sb4
DIM FOO%[3] = [1,2,3]
```

The elements in the array will be assigned to the corresponding elements in the initializer. The length of the array and the number of elements in the initializer must be the same. If no initializer is given, the array's elements are all initialized to their default value (0 for numbers, `""` for strings).

You can omit the array's length if you provide an initializer.

```sb4
' 3 element array
DIM FOO%[] = [1,2,3]
```

The initializer is *not an array literal.* SmileBASIC does not have array literal expressions. You can only use initializers in array declarations.

### ARRAY Functions

There is also a family of functions that return new arrays. They can be used to create new arrays on demand and assign them to existing variables.

```sb4
VAR FOO%, FOO#, FOO$
FOO% = ARRAY%(1)  'int array
FOO# = ARRAY#(1)  'real array
FOO$ = ARRAY$(1)  'string array
```

There is no function just named `ARRAY`. This is another reason to always use type suffixes with arrays: it is more consistent.

Array declarations are really just shorthand for variable declarations initialized with these `ARRAY` functions.

```sb4
DIM FOO%[1]
'is the same as
VAR FOO% = ARRAY%(1)
```

```sb4
DIM FOO%[3] = [1,2,3]
'is the same as
VAR FOO% = ARRAY%(3)
FOO%[0] = 1
FOO%[1] = 2
FOO%[2] = 3
```

This should demonstrate why arrays aren't really any special: they're just values like anything else. Use both ways of creating and initializing arrays where they're most convenient and appropriate.

### Empty Arrays

You can also create *empty arrays*. These arrays have no elements or valid indices.

```sb4
DIM ARRAY%[0]
DIM ARRAY%[]   'you can omit the length if you want
VAR ARRAY%=ARRAY%(0)
```

This may not seem useful, but you'll see why it is later.

## Length

The *length* of an array is the *total number of elements* it contains. You can check what the length of an array is using the `LEN` function.

```sb4
DIM ARRAY%[3]
PRINT LEN(ARRAY%)  '3
```

## Indexing

Using the `[brackets]` to access an element of the array is called *indexing*. The indices of an array start at 0 and end at the array's length minus one.

### Assignment

You can store a value in an element using an assignment. Remember, an array can only store a certain type of value!

```sb4
DIM FOO%[3]
FOO%[0]=10
'this is truncated to an int 10
FOO%[0]=10.2
'this doesn't even work!
FOO%[0]="10"
```

### Access

Indexing an array in an expression returns the value of that element.

```sb4
PRINT FOO%[0]  '10
```

You can even swap two elements.

```sb4
SWAP FOO%[0],FOO%[1]
```

You cannot access an element at an index that does not exist.

```sb4
PRINT FOO%[4]   'foo is not that long
PRINT FOO%[-1]  'negative indices are invalid
'assigning an out of bounds index does not create it
FOO%[4] = 10
```

### LAST

The `LAST` function tells you what the index of the last value in the array is. This is especially convenient in `FOR` loops.

```sb4
FOR I=0 TO LAST(FOO%)
 PRINT FOO%[I]
NEXT I
```

## Dimensions

An array can contain multiple *dimensions*. This is like assigning multiple "coordinates" to each element of the array. Multi-dimensional arrays are created by specifying multiple lengths. An array can have up to four dimensions.

```sb4
'create 2D arrays
DIM FOO%[3,3]
DIM BAR$ = ARRAY$(3,3)
```

The length of a multidimensional array is the *product* of all of its dimensions. So `FOO%` contains `3*3 = 9` elements.

```sb4
PRINT LEN(FOO%)  '9
PRINT LAST(FOO%)  '8
```

You *cannot* create empty multi-dimensional arrays. If any dimension is zero, you will get a `Subscript out of range` error.

```sb4
DIM FOO%[0,0] 'no
DIM FOO%[5,0] 'not it either
VAR FOO%=ARRAY%(0,0)  'stil no!
```

You can also use initializers with multi-dimensional arrays.

```sb4
DIM FOO%[3,3] = [1,2,3,4,5,6,7,8,9]
```

If you picture a 2D array as a grid of values, each element and its indices are arranged like this:

| y/x | 0 | 1 | 2 |
| --- | --- | --- | --- |
| 0 | 1 | 2 | 3 |
| 1 | 4 | 5 | 6 |
| 2 | 7 | 8 | 9 |

`FOO%[0,0]` is 1, `FOO%[2,1]` is 8, etc.

### Element Order

Array dimensions increase from right to left ("last" to "first"). This is called a [row-major order](https://en.wikipedia.org/wiki/Row-_and_column-major_order). This is why the values in the initializer are assigned in the order they are. If we "de-sugar" this declaration, it looks like this:

```
VAR FOO% = ARRAY%(3,3)
FOO%[0,0] = 1
FOO%[0,1] = 2
FOO%[0,2] = 3
FOO%[1,0] = 4
FOO%[1,1] = 5
FOO%[1,2] = 6
FOO%[2,0] = 7
FOO%[2,1] = 8
FOO%[2,2] = 9
```

This section will mainly demonstrate 2D arrays, because thinking of a table of values is familiar, but thinking of a cube or 4D volume of values is not. Just know that an array can have at most four dimensions.

### Multidimensional Indexing

Indexing a multi-dimensional array works like you expect.

```sb4
FOO%[1,2] = 10
PRINT FOO%[1,2]
```

### Checking Dimensions

As stated before, `LEN` only tells you the total length of the array. It doesn't tell you how many dimensions it has, or the length of each of those dimensions. Instead, a function named `DIM` does that.

> The `DIM` *function* is not the same as the `DIM` *keyword* used to declare arrays. This might seem confusing, so keep it in mind.

Calling `DIM` with just the array as its argument will return the number of dimensions in the array. (This is sometimes called its *rank*.)

```sb4
DIM FOO%[3,3]
PRINT DIM(FOO%)  '2
```

If you pass a dimension number as the second argument, the function will return the length of that dimension. These are zero-indexed as well. 0 is the "first" dimension, and so on.

```sb4
DIM FOO%[5,2,3]
FOR I=0 TO DIM(FOO%)-1
 PRINT DIM(FOO%,I)
NEXT I
```

The above code should print:

```none
5
2
3
```

> Sometimes, the lengths of the dimensions are together called the array's *shape*, and each dimension is called an *axis*. These terms are not common in SmileBASIC, but they are used in other places.

### Handling Multi-Dimensional Arrays as Linear Arrays

Any multi-dimensional array can be indexed as a 1D (or "linear") array.

```sb4
FOO%[5] = 10
PRINT FOO%[5]
```

The index value used is based on the row-major order described above: the last index always counts first. This is actually the order used to store arrays in memory (because all computer memory is linear). If we go back to our de-sugared declaration, it *really* looks like this:

```sb4
VAR FOO% = ARRAY%(3,3)
FOO%[0] = 1
FOO%[1] = 2
FOO%[2] = 3
FOO%[3] = 4
FOO%[4] = 5
FOO%[5] = 6
FOO%[6] = 7
FOO%[7] = 8
FOO%[8] = 9
```

It's not often that you'll be indexing multi-dimensional arrays as 1D if you also know what the dimensions are. This is only important in contexts where you *don't know* or *don't care* what the array's dimensions are. For instance, you can write a function that takes the sum of every value in an array, and it will work with any number of dimensions.

```sb4
DEF SUM(ARY[])
 IF LEN(ARY) == 0 THEN RETURN

 VAR ACC = ARY[0]
 VAR I
 FOR I=1 TO LAST(ARY)
  INC ACC,ARY[I]
 NEXT I
 RETURN ACC
END
```

However, some built-in functions treat all arrays as linear, or only create linear arrays. Others behave differently if an array is linear or multi-dimensional. Linear arrays and multidimensional arrays are necessarily different, and the number of dimensions in an array can never change.

## Resizing

At any time, the first dimension of an an array can be resized using the `RESIZE` function.

```sb4
DIM FOO%[0]
RESIZE FOO%,5
PRINT LEN(FOO%)
```

If you shrink an array, the elements at the end are lost.

```sb4
DIM FOO%[5] = [1,2,3,4,5]
RESIZE FOO%,3
INSPECT FOO%
```

If you resize a multi-dimensional array, you must specify all dimensions; but remember, you can only change the first one!

```sb4
DIM FOO%[3,3]
RESIZE FOO%,4,3  'ok
RESIZE FOO%,4,4  'bad!
```

It may seem annoying that you have to specify dimensions that you can't even change. It is. It's completely unnecessary, but those are SmileBoom's rules.

You also cannot change the number of dimensions in an array.

```sb4
DIM FOO%[3]
RESIZE FOO%,3,3  'no!
```

Strangely, though, you *can* make the first dimension of a multi-dimensional array 0 using `RESIZE`. You aren't allowed to *initialize* them with a zero first dimension, but you can *resize* it that way. Odd. This array is empty, so you can't store anything in it, but it keeps this dangling "phantom dimension".

```sb4
DIM FOO%[3,3]
RESIZE FOO%,0,3
INSPECT FOO%
```

These rules might seem arbitrary, and they are in a way, but they were chosen for two reasons:

- consistency
- efficiency.

Because of the row-major memory layout, these operations are exactly the same for all ranks of arrays. Only the first dimension will ever change, which is of course the "most significant" dimension. If elements have to be removed, they are all in a contiguous block of memory at the end, and if elements have to be added, they are always added at the end. This is always true, linear array or otherwise. If you were to resize other dimensions, most of the array's memory layout would be rearranged, and it may not be obvious where elements are inserted or removed.

## Inserting and Removing Elements

In a linear array, elements can be inserted and removed from any index. This is not allowed for multi-dimensional arrays, for the reasons stated previously.

> pal, you gotta write refpages for these functions instead, so you avoid duping information. but first, you need function boxes!
