---
title: MAX
slug: docs-sb3-max
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# MAX

> **Category:** Mathematics

## MAX (1)

Gets the largest value in the specified numerical value array

### Format

```sb3
Variable = MAX( Numerical value array )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical value<br>array` | Name of a numerical value array storing multiple numerical values |

### Return Values

Largest number in the passed arguments

### Examples

```sb3
DIM TMP[2]
TMP[0]=50:TMP[1]=3
A=MAX(TMP)
```

## MAX (2)

Gets the largest value from the specified multiple numerical values

### Format

```sb3
Variable = MAX( Numerical value [,Numerical value …
] )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical values<br>enumerated<br>directly` | Enumerate multiple numerical values separated by commas |

### Return Values

Largest number in the passed arguments

### Examples

```sb3
A=MAX(1,2,3,4)
```
