---
title: BGROT
slug: docs-sb3-bgrot
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGROT

> **Category:** BG

## BGROT (1)

Rotates the BG screen

### Format

```sb3
BGROT Layer,Angle
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Angle` | Rotation angle (clockwise): 0-360 |

### Examples

```sb3
BGROT 0,180
```

## BGROT (2)

Gets rotation information from the BG screen

### Format

```sb3
BGROT Layer OUT R
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |

### Return Values

| Return Value | Description |
| --- | --- |
| `Angle` | R: 0-360 |

### Examples

```sb3
BGROT 0 OUT R
```
