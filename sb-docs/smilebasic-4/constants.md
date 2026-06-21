---
title: Built-In Constants
slug: docs-sb4-constants
system: SmileBASIC 4
type: reference
source: https://smilebasicsource.com/forum/thread/docs-sb4-constants
content_id: 19449
created: 2020-04-26
scraped: 2026-06-21
---

# Built-In Constants

## General

### Booleans

| Name | Value |
| --- | --- |
| `#TRUE`, `#YES`, `#ON` | 1 |
| `#FALSE`, `#NO`, `#OFF` | 0 |

### `TYPEOF`

| Name | Value | Description |
| --- | --- | --- |
| `#T_DEFAULT` | 0 | Default/empty value |
| `#T_INT` | 1 | Integer |
| `#T_REAL` | 2 | Real number (double) |
| `#T_STR` | 3 | String |
| | 4 | /unused/ |
| `#T_INTARRAY` | 5 | Array of integers |
| `#T_REALARRAY` | 6 | Array of real numbers |
| `#T_STRARRAY` | 7 | Array of strings |

### Special

| Name | Value |
| --- | --- |
| `#HARDWARE` | device ID, currently always 10 for Switch/Lite |
| `#VERSION` | current version, ex: 4.2.1 = `4020100` |
| `#_LINE` | current line number |
| `#_SLOT` | current slot number |
| `#_FILENAME` | current file name |

## Buttons

| Name | Value | Description |
| --- | --- | --- |
| `#B_RUP`, `#B_X` | 0 | Top face button / X |
| `#B_RDOWN`, `#B_B` | 1 | Bottom face button / B |
| `#B_RLEFT`, `#B_Y` | 2 | Left face button / Y |
| `#B_RRIGHT`, `#B_A` | 3 | Right face button / A |
| `#B_LUP` | 4 | D-Pad up |
| `#B_LDOWN` | 5 | D-Pad down |
| `#B_LLEFT` | 6 | D-Pad left |
| `#B_LRIGHT` | 7 | D-Pad right |
| `#B_L1`, `#B_SL` | 8 | L trigger / SL trigger |
| `#B_R1`, `#B_SR` | 9 | R trigger / SR trigger |
| `#B_L2`, `#B_S1` | 10 | ZL trigger / Joy-Con side trigger |
| `#B_R2`, `#B_S2` | 11 | ZR trigger / Joy-Con side Z trigger |
| `#B_LSTICK` | 12 | Left stick click |
| `#B_RSTICK` | 13 | Right stick click |
| `#B_RANY` | 14 | Any right side button |
| `#B_LANY` | 15 | Any left side button |
| `#B_ANY` | 16 | Any button |

## Graphics

### Resources

| Name | Value | Description |
| --- | --- | --- |
| `#GRPWIDTH` | 2048 | Width of GRPs, in pixels |
| `#GRPHEIGHT` | 2048 | Height of GRPs, in pixels |
| `#GRPF` | 5 | ID of GRP containing the font |
| `#GSPRITE` | 4095 | ID of sprite initialized to graphics screen by default |
| `#TCONSOLE` | 4 | ID of text screen used for the console (by `PRINT` etc.) |
| `#MAXT` | 3 | Last text screen ID (excluding `#TCONSOLE`) |
| `#MAXSP` | 4094 | Last sprite ID (excluding `#GSPRITE`) |
| `#MAXGRP` | 4 | Last GRP ID (excluding `#GRPF`) |
| `#TUSRCHR` | `&h E800` | Start of BG characters |

### Colors

| Name | Alpha | Red | Green | Blue | Color | Hex |
| --- | --- | --- | --- | --- | --- | --- |
| `#C_CLEAR` | 0 | 0 | 0 | 0 | | `&h 00000000` |
| `#C_BLACK` | 255 | 0 | 0 | 0 | #c=#000000 | `&h FF000000` |
| `#C_GRAY` | 255 | 128 | 128 | 128 | #c=#808080 | `&h FF808080` |
| `#C_SILVER` | 255 | 192 | 192 | 192 | #c=#C0C0C0 | `&h FFC0C0C0` |
| `#C_WHITE` | 255 | 255 | 255 | 255 | #c=#FFFFFF | `&h FFFFFFFF` |
| `#C_RED` | 255 | 255 | 0 | 0 | #c=#FF0000 | `&h FFFF0000` |
| `#C_YELLOW` | 255 | 255 | 255 | 0 | #c=#FFFF00 | `&h FFFFFF00` |
| `#C_LIME` | 255 | 0 | 255 | 0 | #c=#00FF00 | `&h FF00FF00` |
| `#C_CYAN`, `#C_AQUA` | 255 | 0 | 255 | 255 | #c=#00FFFF | `&h FF00FFFF` |
| `#C_BLUE` | 255 | 0 | 0 | 255 | #c=#0000FF | `&h FF0000FF` |
| `#C_MAGENTA`, `#C_FUCHSIA` | 255 | 255 | 0 | 255 | #c=#FF00FF | `&h FFFF00FF` |
| `#C_MAROON` | 255 | 128 | 0 | 0 | #c=#800000 | `&h FF800000` |
| `#C_OLIVE` | 255 | 128 | 128 | 0 | #c=#808000 | `&h FF808000` |
| `#C_GREEN` | 255 | 0 | 128 | 0 | #c=#008000 | `&h FF008000` |
| `#C_TEAL` | 255 | 0 | 128 | 128 | #c=#008080 | `&h FF008080` |
| `#C_NAVY` | 255 | 0 | 0 | 128 | #c=#000080 | `&h FF000080` |
| `#C_PURPLE` | 255 | 128 | 0 | 128 | #c=#800080 | `&h FF800080` |

### Drawing Modes

| Name | Value | Description |
| --- | --- | --- |
| `#G_NORMAL` | 0 | Overwrite all (default) |
| `#G_NORMAL2` | 1 | Overwrite where alpha > 0 |
| `#G_ALPHA` | 2 | Alpha blend treating background as opaque, using background alpha |
| `#G_ALPHA2` | 3 | Full alpha blending |
| `#G_ADD` | 4 | Additive blending |

### Attributes

| Name | Value | Description |
| --- | --- | --- |
| `#A_ROT0` | 0 | No rotation |
| `#A_ROT90` | 1 | Rotate 90 degrees |
| `#A_ROT180` | 2 | Rotate 180 degrees |
| `#A_ROT270` | 3 | Rotate 270 degrees |
| `#A_REVH` | 4 | Flip horizontally |
| `#A_REVV` | 8 | Flip vertically |
| `#A_ADD` | 16 | Additive blending mode |

### Animation Status Flags

| Name | Value | Description |
| --- | --- | --- |
| `#CHKXY` | 1 | XY coordinates |
| `#CHKZ` | 2 | Z offset |
| `#CHKR` | 4 | Rotation |
| `#CHKS` | 8 | XY scale factor |
| `#CHKC` | 16 | Color filter |
| `#CHKV` | 32 | Sprite/text screen variable 7 |
| `#CHKUV` | 64 | Sprite image UV coordinates |
| `#CHKI` | 128 | Sprite definition number |

## Audio

### `EFCEN`

| Name | Value | Description |
| --- | --- | --- |
| `#EFCON` | 1 | Enable effector |
| `#EFCOFF` | 0 | Disable effector |

### Effector Presets

| Name | Value | Description |
| --- | --- | --- |
| `#EFCBATH` | 0 | "Bathroom" effect |
| `#EFCCAVE` | 1 | "Cave" effect |
| `#EFCSPACE` | 2 | "Space" effect |

### Sound Reflection Mode

| Name | Value | Description |
| --- | --- | --- |
| `#EFCREFSROOM` | 0 | "Small Room" reflection mode |
| `#EFCREFLROOM` | 1 | "Large Room" reflection mode |
| `#EFCREFHALL` | 2 | "Hall" reflection mode |
| `#EFCREFCAVE` | 3 | "Cave" reflection mode |
| `#EFCREFNONE` | 4 | No sound reflections |

### Reverb Presets

| Name | Value | Description |
| --- | --- | --- |
| `#EFCREVROOM` | 0 | "Room" reverb effect |
| `#EFCREVHALL` | 1 | "Hall" reverb effect |
| `#EFCREVMETAL` | 2 | "Metallic Corridor" reverb effect |
| `#EFCREVCAVE` | 3 | "Cave" reverb effect |
| `#EFCREVREV` | 4 | "Reverb" reverb effect |

### PCMVOL Channels

| Name | Value | Description |
| --- | --- | --- |
| `#PVLEFT` | 0 | Left PCM channel |
| `#PVRIGHT` | 1 | Right PCM channel |

## Math

### Constants

| Name | Value | Description |
| --- | --- | --- |
| `#PI` | 3.141592653589793 | Pi π |
| `#EXP` | 2.718281828459045 | Euler's number /e/ |

### `BQPARAM` Modes

| Name | Value | Description |
| --- | --- | --- |
| `#BQAPF` | 0 | All pass filter |
| `#BQLPF` | 1 | Low pass filter |
| `#BQHPF` | 2 | High pass filter |
| `#BQBPF` | 3 | Band pass filter |
| `#BQBSF` | 4 | Band stop filter |
| `#BQLSF` | 5 | Low shelf filter |
| `#BQHSF` | 6 | High shelf filter |
| `#BQPEQ` | 7 | Peaking equalizer |

### `FFTWFN` Window Functions

| Name | Value | Description |
| --- | --- | --- |
| `#WFRECT` | 0 | Rectangular window function |
| `#WFHAMM` | 1 | Hamming window function |
| `#WFHANN` | 2 | Hann window function |
| `#WFBLKM` | 3 | Blackman window function |

### `ARYOP` Operations

| Name | Value | Description |
| --- | --- | --- |
| `#AOPADD` | 0 | Add: `p1+p2` |
| `#AOPSUB` | 1 | Subtract: `p1-p2` |
| `#AOPMUL` | 2 | Multiply: `p1*p2` |
| `#AOPDIV` | 3 | Divide: `p1/p2` |
| `#AOPMAD` | 4 | Multiply-add: `p1*p2+p3` |
| `#AOPLIP` | 5 | Linear interpolate: `p1*p3+p2*(1-p3)` |
| `#AOPCLP` | 6 | Clamp: `p2<=p1<=p3` |
