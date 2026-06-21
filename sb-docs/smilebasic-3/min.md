---
title: MIN
slug: docs-sb3-min
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# MIN

> **Category:** Mathematics

## MIN (1)

Gets the smallest value in the specified numerical value array

### Format

```sb3
Variable = MIN( Numerical value array )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical value<br>array` | Name of a numerical value array storing multiple numerical values |

### Return Values

Smallest number in the passed arguments

### Examples

```sb3
DIM TMP[2]
TMP[0]=50:TMP[1]=3
A=MIN(TMP)
```

## MIN (2)

Gets the smallest value from the specified multiple numerical values

### Format

```sb3
Variable = MIN( Numerical value [,Numerical value …
] )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical values<br>enumerated<br>directly` | Enumerate multiple numerical values separated by commas |

### Return Values

Smallest number in the passed arguments

### Examples

```sb3
A=MIN(1,2,3,4)
```
