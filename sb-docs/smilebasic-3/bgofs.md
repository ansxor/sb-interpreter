---
title: BGOFS
slug: docs-sb3-bgofs
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGOFS

> **Category:** BG

## BGOFS (1)

Changes the display offset of the BG screen

### Format

```sb3
BGOFS Layer,X,Y,[Z]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `X,Y` | Display offset coordinates in pixels |
| `Z` | Coordinate in the depth direction (Rear:1024 < Screen surface:0 < Front:-256) |

### Examples

```sb3
BGOFS 0,-100,-100
```

## BGOFS (2)

Gets BG coordinates

### Format

```sb3
BGOFS Layer OUT X,Y[,Z]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |

### Return Values

| Return Value | Description |
| --- | --- |
| `X,Y` | Variables to receive the coordinates |
| `Z` | Variable to receive the depth information |

### Examples

```sb3
BGOFS 0 OUT X,Y,Z
```
