---
title: FONTDEF
slug: docs-sb3-fontdef
system: SmileBASIC 3
type: command
category: Console input/output
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# FONTDEF

> **Category:** Console input/output

## FONTDEF (1)

Defines a font for the specified character code

### Format

```sb3
FONTDEF Character code, "Font definition string"
```

### Arguments

| Argument | Description |
| --- | --- |
| `Character code` | Character code (UTF-16) for which to define a font |
| `Font definition<br>string` | - One pixel corresponds to a 16-bit color code in the RGBA=5551 format<br>- 5 bits for each RGB color (0-31) + alpha channel 1 bit (0: Transparent, 1: Opaque)<br>- A color element should be handled as a 4-digit hexadecimal string<br>- Example) White: FFFF, Black: 0001, Red: F801<br>- As one character occupies 8x8=64 pixels, its font definition string should consist of a<br>total of 256 characters |

### See Also

Font images can be manipulated with GCOPY, GSAVE, or GLOAD page number -1

### Examples

```sb3
F$="FFFF":Z$="0000"
D$=F$*7+Z$
D$=D$+F$*2+Z$*3+F$*2+Z$
D$=D$+F$+Z$+F$*3+Z$+F$+Z$
D$=D$+F$+Z$*5+F$+Z$
D$=D$+F$+Z$+F$*3+Z$+F$+Z$
D$=D$+F$+Z$+F$*3+Z$+F$+Z$
D$=D$+F$*7+Z$
D$=D$+Z$*8
FONTDEF ASC("A"),D$
```

## FONTDEF (2)

Defines a font for the specified character code

### Format

```sb3
FONTDEF Character code, Numerical value array
```

### Arguments

| Argument | Description |
| --- | --- |
| `Character code` | Character code (UTF-16) for which to define a font |
| `Font definition<br>array` | - A numerical value array with an element for each pixel should be prepared (8x8 pixels for<br>one character = 64 elements)<br>- One pixel corresponds to a 16-bit color code in the RGBA=5551 format<br>- 5 bits for each RGB color (0-31) + alpha channel 1 bit (0: Transparent, 1: Opaque)<br>- Example) White: &HFFFF, Black: &H0001, Red: &HF801 |

### See Also

Font images can be manipulated with GCOPY, GSAVE, or GLOAD page number -1

### Examples

```sb3
DIM F%[64]
DATA "11111110"
DATA "11000110"
DATA "10111010"
DATA "10000010"
DATA "10111010"
DATA "10111010"
DATA "11111110"
DATA "00000000"
TOP=ASC("A"):CNT=1
FOR I=0 TO CNT-1
 FOR D=0 TO 7
  READ F$
  FOR B=0 TO 7
   C=0:IF MID$(F$,B,1)=="1" THEN C=&HFFFF
   F%[D*8+B]=C
  NEXT
 NEXT
 FONTDEF TOP+I,F%
NEXT
```

## FONTDEF (3)

Resets the font definition to its initial state

### Format

```sb3
FONTDEF
```

### Examples

```sb3
FONTDEF
```
