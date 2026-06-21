---
title: BGVAR
slug: docs-sb3-bgvar
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# BGVAR

> **Category:** BG

## BGVAR (1)

Writes to a BG internal variable User variables; there are eight variables for each BG layer

### Format

```sb3
BGVAR Layer,Internal variable number,Numerical value
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |
| `Numerical value` | Numerical value to register with the internal variable |

### Examples

```sb3
BGVAR 0,7,1
```

## BGVAR (2)

Reads a BG internal variable (function type) User variables; there are eight variables for each BG layer

### Format

```sb3
Variable=BGVAR( Layer number,Internal variable number )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |

### Return Values

Value written with BGVAR

### Examples

```sb3
V=BGVAR(0,5)
```

## BGVAR (3)

Reads a BG internal variable

- User variables; there are eight variables for each BG layer

### Format

```sb3
BGVAR Layer,Internal variable number OUT V
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |

### Return Values

| Return Value | Description |
| --- | --- |
| `V` | Numerical value variable that returns the value of the internal variable |

### Examples

```sb3
BGVAR 0,5 OUT V
```
