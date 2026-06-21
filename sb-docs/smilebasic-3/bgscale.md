---
title: BGSCALE
slug: docs-sb3-bgscale
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGSCALE

> **Category:** BG

## BGSCALE (1)

Scales the BG screen

- When scaled down, BGs exceeding 3600 in total will not be displayed
- If this display limit is exceeded, the BG screen will be distorted

### Format

```sb3
BGSCALE Layer,Enlargement scale X,Enlargement scale Y
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Enlargement scale<br>X,Y` | 0.5 (50%) - 1.0 (100%) - 2.0(200%) - |

### Examples

```sb3
BGSCALE 0,1.5,2.0
```

## BGSCALE (2)

Gets scale-up/down information from the BG screen

### Format

```sb3
BGSCALE Layer OUT SX,SY
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |

### Return Values

| Return Value | Description |
| --- | --- |
| `SX,SY` | 0.5 (50%) - 1.0 (100%) - 2.0(200%) - |

### Examples

```sb3
BGSCALE 0 OUT SX,SY
```
