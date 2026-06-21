---
title: SPCHK
slug: docs-sb3-spchk
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# SPCHK

> **Category:** Sprites

Gets the animation status of a sprite If used before SPSET, an error will occur

## Format

```sb3
Variable=SPCHK( Management number )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the target sprite: 0-511 |

## Return Values

```
|b00| XY-coordinates (1), #CHKXY
|b01| Z-coordinates (2), #CHKZ
|b02| UV-coordinates (4), #CHKUV
|b03| Definition number (8), #CHKI
|b04| Rotation (16), #CHKR
|b05| Magnification XY (32), #CHKS
|b06| Display color (64), #CHKC
|b07| Variable (128), #CHKV
For each bit, a target is assigned (If 0 is assigned for all bits, animation is being stopped)
```

## Examples

```sb3
ST=SPCHK(5)
'|b00|#CHKXY
'|b01|#CHKZ
'|b02|#CHKUV
'|b03|#CHKI
'|b04|#CHKR
'|b05|#CHKS
'|b06|#CHKC
'|b07|#CHKV
```
