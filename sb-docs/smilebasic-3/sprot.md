---
title: SPROT
slug: docs-sb3-sprot
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SPROT

> **Category:** Sprites

## SPROT (1)

Specifies the rotation angle of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPROT Management number,Angle
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Angle` | Rotation angle: 0-360 (clockwise) |

### Examples

```sb3
SPROT 23,45
```

## SPROT (2)

Gets the rotation angle of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPROT Management number OUT DR
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `DR` | Variable to receive the angle |

### Examples

```sb3
SPROT 23 OUT DR
```

## SPROT (3)

Gets the rotation angle of a sprite (Function type) If used before SPSET, an error will occur

### Format

```sb3
Variable=SPROT(Management number)
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

```
Current angle (0-360)
```

### Examples

```sb3
A=SPROT(23)
```
