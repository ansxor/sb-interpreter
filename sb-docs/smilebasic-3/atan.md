---
title: ATAN
slug: docs-sb3-atan
system: SmileBASIC 3
type: command
category: Mathematics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# ATAN

> **Category:** Mathematics

## ATAN (1)

Returns the arc tangent value (from numerical values)

### Format

```sb3
Variable = ATAN( Numerical value )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical value` | Numerical value from which to find the angle |

### Return Values

```
Arc tangent (radian) value found
```

### Examples

```sb3
A=ATAN(1)
```

## ATAN (2)

Returns the arc tangent value (from XY-coordinates)

### Format

```sb3
Variable = ATAN( Y-coordinate,X-coordinate )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Y-,X-coordinates` | - X-,Y-coordinates from the origin<br>- The Y-coordinate should be input first |

### Return Values

```
Arc tangent (radian) value found
```

### Examples

```sb3
A=ATAN(1,1)
```
