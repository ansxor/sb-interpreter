---
title: VAR
slug: docs-sb3-var
system: SmileBASIC 3
type: command
category: Basic instructions (variables and arrays)
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# VAR

> **Category:** Basic instructions (variables and arrays)

## VAR (1)

Declares variables to use When OPTION STRICT is specified, each variable that will be used must be declared

### Format

```sb3
VAR Variable name ,
…
```

### Arguments

| Argument | Description |
| --- | --- |
| `Variable name` | - Alphanumeric characters and underscores (_) are allowed<br>- Leading numerals are not allowed<br>- String variables can also be declared |

### Examples

```sb3
VAR A, ATRB, B$
```

## VAR (2)

Declares arrays to use

- In this product, arrays must always be declared
- The subscript should begin with 0
- The number of elements must be enclosed in []. () is not allowed
- Either DIM or VAR can be used

### Format

```sb3
VAR Array variable name[ Number of elements ] , …
```

### Arguments

| Argument | Description |
| --- | --- |
| `Array variable<br>name[ Number of<br>elements ]` | - Alphanumeric characters and underscores (_) are allowed<br>- Leading numerals are not allowed<br>- String variables are also allowed for the array variable |
| `Number of elements` | - Specify the number of array elements to provide, enclosed in []<br>- Up to four dimensions can be specified, with commas (,) to separate them |

### Examples

```sb3
VAR ATR[4]
VAR DX[5], DY[5], DZ[5]
VAR POS[10, 5]
```
