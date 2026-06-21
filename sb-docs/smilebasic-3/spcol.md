---
title: SPCOL
slug: docs-sb3-spcol
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 7
scraped: 2026-06-21
---

# SPCOL

> **Category:** Sprites

## SPCOL (1)

Sets sprite collision detection information

- Must be called before any SPHIT instruction is used
- SPCOLVEC should also be called
- If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number [,Scale adjustment]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Scale adjustment` | FALSE = Ignores this specification (If omitted = FALSE)<br>TRUE = Synchronizes the detection area with SPSCALE<br>* This specification will be effective for SPSCALE instructions set after the SPCOL<br>instruction. |

### Examples

```sb3
SPCOL 3,TRUE
```

## SPCOL (2)

Sets sprite collision detection information (including mask specification)

- Must be called before any SPHIT instruction is used
- SPCOLVEC should also be called
- If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number,[Scale adjustment],Mask
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Scale adjustment` | FALSE = Ignores this specification (If omitted = FALSE)<br>TRUE = Synchronizes the detection area with SPSCALE<br>* This specification will be effective for SPSCALE instructions set after the SPCOL<br>instruction. |
| `Mask` | 0 - &HFFFFFFFF (32 bits)<br>* For collision detection, the AND of the bits is determined,<br>and if it is 0, it is regarded as no collision (If omitted, &HFFFFFFFF). |

### Examples

```sb3
SPCOL 3,TRUE,31
SPCOL 3,,31
```

## SPCOL (3)

Sets sprite collision detection information (including range specification)

- Must be called before any SPHIT instruction is used
- SPCOLVEC should also be called
- If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number,Start point X,Start point Y,Width,Height,[Scale adjustment],Mask
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `Start point X,Y` | - Start point coordinates of the detection area: X,Y (-32768 to 32767)<br>- Relative coordinates with SPHOME as the origin (0,0) |
| `Width,Height` | Width and height of the detection area: W,H (0-65535) |
| `Scale adjustment` | FALSE = Ignores this specification (If omitted = FALSE)<br>TRUE = Synchronizes the detection area with SPSCALE<br>* This specification will be effective for SPSCALE instructions set after the SPCOL<br>instruction. |
| `Mask` | 0 - &HFFFFFFFF (32 bits)<br>* For collision detection, the AND of the bits is determined,<br>and if it is 0, it is regarded as no collision (If omitted, &HFFFFFFFF). |

### Examples

```sb3
SPCOL 3,0,0,32,32,TRUE,255
SPCOL 3,0,0,32,32,,255
```

## SPCOL (4)

Gets sprite collision detection information (scale adjustment and mask) If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number OUT Scale adjustment [,Mask]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `Scale adjustment` | Variable to receive the scale value |
| `Mask` | Variable to receive the mask value |

### Examples

```sb3
SPCOL 3 OUT SC,MSK
```

## SPCOL (5)

Gets sprite collision detection information (range) If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number OUT Start point X,Start point Y,Width,Height
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `Start point X,Y` | Variables to receive the start point coordinates of the detection area |
| `Width,Height` | Variables to receive the width and height of the detection area |

### Examples

```sb3
SPCOL 3 OUT X,Y,W,H
```

## SPCOL (6)

Gets sprite collision detection information (range and scale adjustment) If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number OUT Start point X,Start point Y,Width,Height,Scale adjustment
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `Start point X,Y` | Variables to receive the start point coordinates of the detection area |
| `Width,Height` | Variables to receive the width and height of the detection area |
| `Scale adjustment` | Variable to receive the scale value |

### Examples

```sb3
SPCOL 3 OUT X,Y,W,H,SC
```

## SPCOL (7)

Gets sprite collision detection information (all information) If used before SPSET, an error will occur

### Format

```sb3
SPCOL Management number OUT Start point X,Start point Y,Width,Height,Scale adjustment,Mask
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `Start point X,Y` | Variables to receive the start point coordinates of the detection area |
| `Width,Height` | Variables to receive the width and height of the detection area |
| `Scale adjustment` | Variable to receive the scale value |
| `Mask` | Variable to receive the mask value |

### Examples

```sb3
SPCOL 3 OUT X,Y,W,H,SC,MSK
```
