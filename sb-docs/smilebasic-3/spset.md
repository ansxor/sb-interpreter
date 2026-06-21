---
title: SPSET
slug: docs-sb3-spset
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 6
scraped: 2026-06-21
---

# SPSET

> **Category:** Sprites

## SPSET (1)

Creates a sprite (using a definition template)

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET Management number,Definition number
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Number of the sprite to create: 0-511 |
| `Definition number` | Definition number of the template defined with SPDEF: 0-4095 |

### Examples

```sb3
SPSET 1,500
```

## SPSET (2)

Creates a sprite (using image and other information specified directly) Can be used to set a sprite separately without using the values from SPDEF

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET Management number ,U,V [,W,H] ,Attribute
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Number of the sprite to create: 0-511 |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define (If omitted: 16,16)<br>* U+W and/or V+H values greater than 512 will give an error. |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>\|b01\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>\|b02\|<br>↓<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD<br>If omitted, 0x01 (Only display is set to ON) |

### Examples

```sb3
SPSET 54,0,0,32,32,1
```

## SPSET (3)

Finds an available sprite number and creates a sprite (using a definition template) Finds an available sprite number from the whole range

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET Definition number OUT IX
```

### Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | Definition number of the template defined with SPDEF: 0-4095 |

### Return Values

| Return Value | Description |
| --- | --- |
| `IX` | Variable to receive the generated number: 0-511 (-1 = No available number) |

### Examples

```sb3
SPSET 500 OUT IX
```

## SPSET (4)

Finds an available sprite number and creates a sprite (using image and other information specified directly) Finds an available sprite number from the whole range

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET U,V,W,H,Attribute OUT IX
```

### Arguments

| Argument | Description |
| --- | --- |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define (If omitted: 16,16)<br>* U+W and/or V+H values greater than 512 will give an error. |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>\|b01\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>\|b02\|<br>↓<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD |

### Return Values

| Return Value | Description |
| --- | --- |
| `IX` | Variable to receive the generated number: 0-511 (-1 = No available number) |

### Examples

```sb3
SPSET 0,0,32,32,1 OUT IX
```

## SPSET (5)

Finds an available sprite number in a certain range and creates a sprite (using a definition template) Finds an available number in the specified range

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET Upper limit,Lower limit, Definition number OUT IX
```

### Arguments

| Argument | Description |
| --- | --- |
| `Upper limit, Lower<br>limit` | Range in which to find an available number (0-511) |
| `Definition number` | Definition number of the template defined with SPDEF: 0-4095 |

### Return Values

| Return Value | Description |
| --- | --- |
| `IX` | Variable to receive the generated number: 0-511 (-1 = No available number) |

### Examples

```sb3
SPSET 100,120, 500 OUT IX
```

## SPSET (6)

Finds an available sprite number in a certain range and creates a sprite (using image and other information specified directly) Finds an available number in the specified range

- SPSET makes a sprite available for use
- Executing SPSET will initialize rotation and all other information
- All values of SPVAR will be 0
- When any SPHIT instruction for collision detection is to be used, SPCOL should be called after SPSET

### Format

```sb3
SPSET Upper limit,Lower limit, U,V,W,H,Attribute OUT IX
```

### Arguments

| Argument | Description |
| --- | --- |
| `Upper limit,Lower<br>limit` | Range in which to find an available number (0-511) |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define (If omitted: 16,16)<br>* U+W and/or V+H values greater than 512 will give an error. |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>↑<br>\|b01\|<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>\|b02\|<br>↓<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD<br>If omitted, 0x01 (Only display is set to ON) |

### Return Values

| Return Value | Description |
| --- | --- |
| `IX` | Variable to receive the generated number: 0-511 (-1 = No available number) |

### Examples

```sb3
SPSET 100,120, 0,0,32,32,1 OUT IX
```
