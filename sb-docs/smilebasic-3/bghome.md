---
title: BGHOME
slug: docs-sb3-bghome
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGHOME

> **Category:** BG

## BGHOME (1)

Sets the display origin of a layer

- Origin for rotation and scaling of the BG screen

### Format

```sb3
BGHOME Layer,Position X,Position Y
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target layer number: 0-3 |
| `Position X,Y` | Origin coordinates in pixel units |

### Examples

```sb3
BGHOME 0,200,120
```

## BGHOME (2)

Gets the display origin of a layer

### Format

```sb3
BGHOME Layer OUT HX,HY
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Target origin number: 0-3 |

### Return Values

| Return Value | Description |
| --- | --- |
| `HX,HY` | Variables to receive the coordinates of the reference point |

### Examples

```sb3
BGHOME 0 OUT HX,HY
```
