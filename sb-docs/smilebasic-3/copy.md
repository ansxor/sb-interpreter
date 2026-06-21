---
title: COPY
slug: docs-sb3-copy
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# COPY

> **Category:** Basic instructions (variables and arrays)

## COPY (1)

Copies one array to another array

- For one-dimensional arrays only, if the number of elements in the copy destination is insufficient, the required

element(s) will be added automatically

- Both the copy source and destination ignore dimensions

### Format

```sb3
COPY Copy destination array [,Copy destination offset],Copy source array [[,Copy source offset] , Number of copy
elements]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Copy destination<br>array` | Copy destination array (to be overwritten with the content of the copy source array) |
| `Copy destination<br>offset` | First element to be overwritten (If this is omitted, overwriting will start with the beginning<br>of the copy destination) |
| `Copy source array` | Copy source array |
| `Copy source offset` | First element to be overwritten (If this is omitted, copying will start with the beginning of<br>the copy source) |
| `Number of copy<br>elements` | Number of elements to be overwritten (If this is omitted, up to the end of the copy source<br>will be copied) |

### Examples

```sb3
DIM SRC[10],DST[10]
COPY DST,SRC
```

## COPY (2)

Reads a DATA sequence into an array

- The data elements enumerated in the DATA instruction will be read into the array
- For one-dimensional arrays only, if the number of elements in the copy destination is insufficient, the required

element(s) will be added automatically

### Format

```sb3
COPY Copy destination array [,Copy destination offset], "@Label string" [,Number of copy data items]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Copy destination<br>array` | Copy destination array (to be overwritten with the content of the DATA sequence) |
| `Copy destination<br>offset` | First element to be overwritten (If this is omitted, overwriting will start with the beginning<br>of the array) |
| `"@Label string"` | Specify the @Label name string set to the DATA instruction to be read |
| `Number of copy<br>data items` | - Number of data items to be read (If this is omitted, data items will be read according to<br>the number of elements in the copy destination array)<br>- If the number of data items is smaller than the number of arrays in the copy destination, an<br>error will occur. |

### Examples

```sb3
DIM DST[5]
COPY DST,"@SRC"
@SRC
DATA 5,1,1,2,4
```
