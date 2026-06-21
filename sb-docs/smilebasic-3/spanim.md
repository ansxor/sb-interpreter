---
title: SPANIM
slug: docs-sb3-spanim
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SPANIM

> **Category:** Sprites

## SPANIM (1)

Displays animation with a sprite (using animation data specified with an array) If used before SPSET, an error will occur

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following SPANIM execution
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
SPANIM Management number,"Animation target",Data array [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite for which to set the animation: 0-511 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinates<br>- 2 or "UV": UV-coordinates (Coordinates of the definition source image)<br>- 3 or "I": Definition number<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of sprite internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as being relative<br>to the run time<br>- Suffixing the character string with "+" will also cause the value to be treated as being<br>relative to the run time |
| `Data array` | One-dimensional numerical value array storing the animation data |
| `Loop` | Loop count: (1-) The value 0 specifies an endless loop |

### Data Arrays

| Item | Description |
| --- | --- |
| `Animation data should be provided in a numerical value array in the following order (Up to 32 pieces of data):<br>Time 1, Item 1,[Item2,] Time 2,Item 1,[Item 2,]` | … |

### Examples

```sb3
DIM PANIM[ 6 ]
PANIM[0] = -60 'frame(-60=smooth)
PANIM[1] = 200 'offset X,Y
PANIM[2] = 100
PANIM[3] = -30 'frame
PANIM[4] = 50 'offset
PANIM[5] = 20
SPSET 0,0
SPANIM 0,"XY",PANIM
```

## SPANIM (2)

Displays animation with a sprite (using animation data specified with the DATA instruction) If used before SPSET, an error will occur

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following SPANIM execution
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
SPANIM Management number,"Animation target","@Label string" [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite for which to set the animation: 0-511 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinates<br>- 2 or "UV": UV-coordinates (Coordinates of the definition source image)<br>- 3 or "I": Definition number<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of sprite internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as being relative<br>to the run time<br>- Suffixing the character string with "+" will also cause the value to be treated as being<br>relative to the run time |
| `@Label string` | - First label of the DATA instruction storing the animation data<br>- This should be specified as a character string by enclosing the @Label name in "" (or as a<br>string variable) |
| `Loop` | Loop count: (1-) The value 0 specifies an endless loop |

### Data

```
Animation data should be provided in the DATA instruction in the following order:
DATA Number of key frames (maximum: 32)
DATA Time 1,Item 1[,Item 2]
DATA Time 2,Item 1[,Item 2]
  :
```

### Examples

```sb3
@MOVDATA
DATA 2 'counter
DATA -60,200,100 'frame,offset
DATA -30,50,20 'frame,offset
SPSET 0,0
SPANIM 0,"XY","@MOVDATA"
```

## SPANIM (3)

Displays animation with a sprite (using animation data specified directly as arguments) If used before SPSET, an error will occur

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following SPANIM execution
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
SPANIM Management number,"Animation target",Time 1,Item 1[,Item 2] [,Time 2,Item 1[,Item 2]] …
 [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite for which to set the animation: 0-511 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinates<br>- 2 or "UV": UV-coordinates (Coordinates of the definition source image)<br>- 3 or "I": Definition number<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of sprite internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as being relative<br>to the run time<br>- Suffixing the character string with "+" will also cause the value to be treated as being<br>relative to the run time |
| `Time,Item` | - Animation data itself (As many Time/Item sets as needed should be listed. Maximum: 32) |
| `Loop` | Loop count: (1-) The value 0 specifies an endless loop |

### Examples

```sb3
SPSET 0,0
SPANIM 0,"XY", -60,200,100, -30,50,20
```
