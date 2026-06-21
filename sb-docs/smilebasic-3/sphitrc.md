---
title: SPHITRC
slug: docs-sb3-sphitrc
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SPHITRC

> **Category:** Sprites

## SPHITRC (1)

Detects collision between a moving quadrangle and any sprite

- SPCOL and SPCOLVEC should be called beforehand
- If used before SPSET, an error will occur

### Format

```sb3
SPHITRC( Start point X,Start point Y,Width,Height[,[Mask],Movement amount X,Movement amount Y] )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Start point X,Y` | Top left coordinates of the quadrangle to detect collision with |
| `Width,Height` | Width and height of the quadrangle to detect collision with |
| `Mask` | 0 - &HFFFFFFFF (32 bits)<br>* For collision detection, the AND of the bits is determined,<br>and if it is 0, it is regarded as no collision (If omitted, &HFFFFFFFF). |
| `Movement amount<br>X,Y` | Movement amount of the quadrangle to detect collision with |

### Return Values

```
Management number of the colliding sprite (When no collision, -1)
```

### Examples

```sb3
H=SPHITRC( 0,0,16,16 )
```

## SPHITRC (2)

Detects collision between the specified sprite and a quadrangle

- SPCOL and SPCOLVEC should be called beforehand
- If used before SPSET, an error will occur

### Format

```sb3
SPHITRC( Management number,Start point X,Start point Y,Width,Height[,[Mask],Movement amount X,Movement amount Y] )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the collision opponent sprite: 0-511 |
| `Start point X,Y` | Top left coordinates of the quadrangle to detect collision with |
| `Width,Height` | Width and height of the quadrangle to detect collision with |
| `Mask` | 0 - &HFFFFFFFF (32 bits)<br>* For collision detection, the AND of the bits is determined,<br>and if it is 0, it is regarded as no collision (If omitted, &HFFFFFFFF). |
| `Movement amount<br>X,Y` | Movement amount of the quadrangle to detect collision with |

### Return Values

```
FALSE = No collision, TRUE = Collision
```

### Examples

```sb3
H=SPHITRC( 1,0,0,16,16 )
```

## SPHITRC (3)

Detects collision between the specified range of sprites and a quadrangle

- SPCOL and SPCOLVEC should be called beforehand
- If used before SPSET, an error will occur

### Format

```sb3
SPHITRC( First ID,Last ID, Start point x,Start point y,Width,Height[,[Mask],Movement amount X, Movement amount Y]
)
```

### Arguments

| Argument | Description |
| --- | --- |
| `First ID,Last ID` | Range of sprites to detect (0-511) |
| `Start point X,Y` | Top left coordinates of the quadrangle to detect collision with |
| `Width,Height` | Width and height of the quadrangle to detect collision with |
| `Mask` | 0 - &HFFFFFFFF (32 bits)<br>* For collision detection, the AND of the bits is determined,<br>and if it is 0, it is regarded as no collision (If omitted, &HFFFFFFFF). |
| `Movement amount<br>X,Y` | Movement amount of the quadrangle to detect collision with |

### Return Values

```
Management number of the colliding sprite (When no collision, -1)
```

### Examples

```sb3
H=SPHITRC( 0,0,16,16 )
```
