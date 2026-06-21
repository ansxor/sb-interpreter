---
title: BGANIM
slug: docs-sb3-bganim
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# BGANIM

> **Category:** BG

## BGANIM (1)

Displays animation with BG (using animation data specified with an array)

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following BGANIM
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
BGANIM Layer,"Animation target",Data array [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Number of the layer for which to set the animation: 0-3 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinates<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of BG internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as being relative<br>to the run time<br>- Suffixing the character string with "+" will also cause the value to be treated as being<br>relative to the run time |
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
BGANIM 0,"XY",PANIM
```

## BGANIM (2)

Displays animation using the BG (Specifying animation data with the DATA instruction)

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following BGANIM
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
BGANIM Layer,"Animation target","@Label string" [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Number of the layer for which to set the animation: 0-3 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinate<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of BG internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as being relative<br>to the run time<br>- Suffixing the character string with "+" will also cause the value to be treated as being<br>relative to the run time |
| `@Label string` | - First label of the DATA instruction storing the animation data<br>- This should be specified as a character string by enclosing the @Label name in " (or as a<br>string variable) |
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
BGANIM 0,"XY","@MOVDATA"
```

## BGANIM (3)

Displays animation using the BG (Specifying animation data with arguments directly)

- Animation waits for a specified time, according to the value input
- Animation starts from the frame following BGANIM
- Up to 32 pieces of data will be accepted for each target element
- If a negative value is specified for time, linear interpolation from the previous value will occur

### Format

```sb3
BGANIM Layer,"Animation target",Time 1,Item 1[,Item 2] [,Time 2,Item 1[,Item 2]] …
 [,Loop]
```

### Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Number of the layer for which to set the animation: 0-3 |
| `Animation target` | Numerical value or character string to control the elements that should change<br>- 0 or "XY": XY-coordinates<br>- 1 or "Z": Z-coordinate<br>- 4 or "R": Rotation angle<br>- 5 or "S": Magnification XY<br>- 6 or "C": Display color<br>- 7 or "V": Variable (Value of BG internal variable 7)<br>- Adding 8 to the target numerical value will cause the value to be treated as relative to the<br>run time<br>- Suffixing the character string with "+" will also cause the value to be treated as relative<br>to the run time/td> |
| `Time, Item` | - Animation data itself (Up to 32 necessary data items can be listed) |
| `Loop` | Loop count: (1-) The value 0 specifies an endless loop |

### Examples

```sb3
BGANIM 0,"XY", -60,200,100, -30,50,20
```
