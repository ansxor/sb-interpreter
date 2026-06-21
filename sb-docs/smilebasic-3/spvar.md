---
title: SPVAR
slug: docs-sb3-spvar
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SPVAR

> **Category:** Sprites

## SPVAR (1)

Writes to a sprite internal variable

- Sprite internal variables (Each sprite has eight variables that the user can use)
- Can also be used before SPSET (When SPSET is executed, all eight variables will be 0)

### Format

```sb3
SPVAR Management number,Internal variable number,Numerical data
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |
| `Numerical value` | Numerical value to register with the internal variable (0- |

### Examples

```sb3
SPVAR 0,7,1
```

## SPVAR (2)

Reads a sprite internal variable (Function type)

- Sprite internal variables (Each sprite has eight variables that the user can use)
- Can also be used before SPSET (When SPSET is executed, all eight variables will be 0)

### Format

```sb3
Variable=SPVAR( Management number,Internal variable number )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |

### Return Values

Value written with SPVAR

### Examples

```sb3
V=SPVAR(54,0)
```

## SPVAR (3)

Reads a sprite internal variable

- Sprite internal variables (Each sprite has eight variables that the user can use)
- Can also be used before SPSET (When SPSET is executed, all eight variables will be 0)

### Format

```sb3
SPVAR Management number,Internal variable number OUT V
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Internal variable<br>number` | Number of the internal variable: 0-7 |

### Return Values

| Return Value | Description |
| --- | --- |
| `V` | Numerical value variable that returns the value of the internal variable |

### Examples

```sb3
SPVAR 54,0 OUT V
```
