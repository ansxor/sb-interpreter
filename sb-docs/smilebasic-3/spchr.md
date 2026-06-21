---
title: SPCHR
slug: docs-sb3-spchr
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 4
scraped: 2026-06-21
---

# SPCHR

> **Category:** Sprites

## SPCHR (1)

Changes the character definition of a sprite (using the specified template) If used before SPSET, an error will occur

### Format

```sb3
SPCHR Management number, Definition number
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to change the definition of: 0-511 |
| `Definition number` | Definition number of the template registered using the SPDEF instruction: 0-4095 |

### Examples

```sb3
SPCHR 0,500
```

## SPCHR (2)

Changes the character definition of a sprite (using a definition specified directly)

- Arguments other than the management number can be omitted
- If used before SPSET, an error will occur

### Format

```sb3
SPCHR Management number,[U],[V],[W],[H],[Attribute]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define (If omitted: 16,16)<br>* U+W and/or V+H values bigger than 512 will give an error |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>\|b01\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>↓<br>\|b02\|<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD<br>If omitted, 0x01 (Only display is set to ON) |

### Examples

```sb3
SPCHR 5,64,64,16,16,1
SPCHR 6,,,32,32,1 'UV skip
```

## SPCHR (3)

Gets information on the character definition of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPCHR Management number OUT U,V [,W,H [,A] ]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `U,V` | Variables to store the coordinates of the original image |
| `W,H` | Variables to store the size of the original image |
| `A` | Variables to store the attribute |

### Examples

```sb3
SPCHR 5 OUT U,V,W,H,ATR
```

## SPCHR (4)

Gets the character definition number of a sprite If used before SPSET, an error will occur

### Format

```sb3
SPCHR Management number OUT DEFNO
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

### Return Values

| Return Value | Description |
| --- | --- |
| `DEFNO` | Variable to receive the definition number |

### Examples

```sb3
SPCHR 5 OUT DEFNO
```
