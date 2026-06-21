---
title: Built-In Constants
slug: docs-sb3-constants
system: SmileBASIC 3
type: reference
category: Constants
source: InstructionList.pdf
scraped: 2026-06-21
---

# Built-In Constants

32-bit numerical value definitions prepared in the system. Used instead of a numerical value in order to specify a color or handle a button. Example: `IF BUTTON() AND (#A OR #B) THEN`

## Generic

| Constant | Value |
|----------|-------|
| `#ON` | `1` |
| `#OFF` | `0` |
| `#YES` | `1` |
| `#NO` | `0` |
| `#TRUE` | `1` |
| `#FALSE` | `0` |

## RGB colors

| Constant | Value |
|----------|-------|
| `#AQUA` | `&HFF00F8F8` |
| `#BLACK` | `&HFF000000` |
| `#BLUE` | `&HFF0000FF` |
| `#CYAN` | `&HFF0000F8` |
| `#FUCHSIA` | `&HFFF800F8` |
| `#GRAY` | `&HFF808080` |
| `#GREEN` | `&HFF008000` |
| `#LIME` | `&HFF00F800` |
| `#MAGENTA` | `&HFFF800F8` |
| `#MAROON` | `&HFF800000` |
| `#NAVY` | `&HFF000080` |
| `#OLIVE` | `&HFF808000` |
| `#PURPLE` | `&HFF800080` |
| `#RED` | `&HFFF80000` |
| `#SILVER` | `&HFFC0C0C0` |
| `#TEAL` | `&HFF008080` |
| `#WHITE` | `&HFFF8F8F8` |
| `#YELLOW` | `&HFFF8F800` |

## Text colors

| Constant | Value |
|----------|-------|
| `#TBLACK` | `1` |
| `#TMAROON` | `2` |
| `#TRED` | `3` |
| `#TGREEN` | `4` |
| `#TLIME` | `5` |
| `#TOLIVE` | `6` |
| `#TYELLOW` | `7` |
| `#TNAVY` | `8` |
| `#TBLUE` | `9` |
| `#TPURPLE` | `10` |
| `#TMAGENTA` | `11` |
| `#TTEAL` | `12` |
| `#TCYAN` | `13` |
| `#TGRAY` | `14` |
| `#TWHITE` | `15` |

## Button bits

| Constant | Value |
|----------|-------|
| `#UP` | `&H0001` |
| `#DOWN` | `&H0002` |
| `#LEFT` | `&H0004` |
| `#RIGHT` | `&H0008` |
| `#A` | `&H0010` |
| `#B` | `&H0020` |
| `#X` | `&H0040` |
| `#Y` | `&H0080` |
| `#L` | `&H0100` |
| `#R` | `&H0200` |
| `#ZL` | `&H0800` |
| `#ZR` | `&H1000` |

## Text rotation & reflection (ATTR)

| Constant | Value |
|----------|-------|
| `#TROT0` | `&H00` |
| `#TROT90` | `&H01` |
| `#TROT180` | `&H02` |
| `#TROT270` | `&H03` |
| `#TREVH` | `&H04` |
| `#TREVV` | `&H08` |

## Sprite attributes (SPSET/SPCHR ATTR)

| Constant | Value |
|----------|-------|
| `#SPSHOW` | `&H01` |
| `#SPROT0` | `&H00` |
| `#SPROT90` | `&H02` |
| `#SPROT180` | `&H04` |
| `#SPROT270` | `&H06` |
| `#SPREVH` | `&H08` |
| `#SPREVV` | `&H10` |
| `#SPADD` | `&H20` |

## Background attributes (BG ATTR)

| Constant | Value |
|----------|-------|
| `#BGROT0` | `&H0000` |
| `#BGROT90` | `&H0800` |
| `#BGROT180` | `&H1000` |
| `#BGROT270` | `&H2000` |
| `#BGREVH` | `&H4000` |
| `#BGREVV` | `&H8000` |

## Collision check flags (SPCHK/BGCHK)

| Constant | Value |
|----------|-------|
| `#CHKXY` | `&H01` |
| `#CHKZ` | `&H02` |
| `#CHKUV` | `&H04` |
| `#CHKI` | `&H08` |
| `#CHKR` | `&H10` |
| `#CHKS` | `&H20` |
| `#CHKC` | `&H40` |
| `#CHKV` | `&H80` |
