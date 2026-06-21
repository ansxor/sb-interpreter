---
title: SPDEF
slug: docs-sb3-spdef
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 6
scraped: 2026-06-21
---

# SPDEF

> **Category:** Sprites

## SPDEF (1)

Resets the sprite character definition template to its initial state

### Format

```sb3
SPDEF
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Common Supplement for SPDEF

- The sprite definition template is a common component for both the upper screen and the Touch Screen
- This is provided in order to simplify SPSET definition

### Examples

```sb3
SPDEF
```

## SPDEF (2)

Creates a template for sprite character definition

### Format

```sb3
SPDEF Definition number, U,V [,W,H [,Origin X,Origin Y]] [,Attribute]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | Definition number of the template: 0-4095 |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define If omitted: 16,16<br>* U+W and/or V+H values greater than 512 will give an error. |
| `Origin X,Y` | Reference point for the coordinates of the sprite If omitted: 0,0 |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>\|b01\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>\|b02\|<br>↓<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD<br>If omitted, 0x01 (Only display is set to ON) |

### Examples

```sb3
SPDEF 0,192,352,32,32,16,16,1
```

## SPDEF (3)

Creates templates for sprite character definition collectively from an array

### Format

```sb3
SPDEF Numerical value array
```

### Arguments

| Argument | Description |
| --- | --- |
| `Numerical value<br>array` | Numerical value array containing sprite template data<br>- One sprite template consists of the following 7 elements: U,V,W,H,Origin X,Origin<br>Y,Attribute<br>- The number of elements must be a multiple of 7<br>- A specific number of sprite templates (the number of elements divided by 7) will be defined<br>in order, starting with 0 |

### Examples

```sb3
SPDEF SRCDATA
```

## SPDEF (4)

Creates templates for sprite character definition collectively from a DATA sequence

### Format

```sb3
SPDEF "@Label string"
```

### Arguments

| Argument | Description |
| --- | --- |
| `@Label string` | Label of the DATA instruction that enumerates the sprite template data<br>- The @Label name should be enclosed in "" or specified with a string variable<br>- The first data should be the number of sprites to define, followed by enumeration of the<br>data for each sprite (7 data elements per sprite)<br>- One sprite template consists of the following 7 elements: U,V,W,H,Origin X,Origin<br>Y,Attribute |

### Examples

```sb3
SPDEF "@SRCDATA"
```

## SPDEF (5)

Gets information on a sprite character definition template

### Format

```sb3
SPDEF Definition number OUT U,V [,W,H [,HX,HY]] [,A]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | Definition number of the template: 0-4095 |

### Return Values

| Return Value | Description |
| --- | --- |
| `U,V` | Variables to receive the image coordinates |
| `W,H` | Variable to receive the image size |
| `HX,HY` | Variable to receive the reference point for the sprite coordinates |
| `A` | Variable to receive the attribute |

### Examples

```sb3
SPDEF 2 OUT U,V,ATR
```

## SPDEF (6)

Copies a template for sprite character definition

- Unnecessary elements can be omitted (Separating commas (',') are required)
- Arguments are used to adjust the copied template

### Format

```sb3
SPDEF Definition number,Source definition number,[U],[V],[W],[H],[Origin X],[Origin Y],[Attribute]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Definition number` | Definition number of the template: 0-4095 |
| `Source definition<br>number` | Definition number of the copy source: 0-4095 |
| `U,V` | Coordinates of the original image to define (U: 0-511, V: 0-511) |
| `W,H` | Size of the original image to define If omitted: 16,16<br>* U+W and/or V+H values greater than 512 will give an error. |
| `Origin X,Y` | Reference point for the coordinates of the sprite If omitted: 0,0 |
| `Attribute` | \|b00\| Display (0=OFF, 1=ON) #SPSHOW<br>\|b01\|<br>↑<br>Rotation by 90 degrees (Specified with two bits: b01 and b02)<br>\|b02\|<br>↓<br>#SPROT0, #SPROT90, #SPROT0180, #SPROT270<br>\|b03\| Horizontal inversion (0=OFF, 1=ON), #SPREVH<br>\|b04\| Vertical inversion (0=OFF, 1=ON), #SPREVV<br>\|b05\| Additive synthesis (0=OFF, 1=ON), #SPADD<br>If omitted, 0x01 (Only display is set to ON) |

### Examples

```sb3
SPDEF 0,255,192,352,32,32,16,16,1
SPDEF 1,255,,,32,32,,,
```
