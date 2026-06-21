---
title: BGCHK
slug: docs-sb3-bgchk
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# BGCHK

> **Category:** BG

Gets BG animation status

## Format

```sb3
Variable=BGCHK( Layer )
```

## Arguments

| Argument | Description |
| --- | --- |
| `Layer` | Number of the layer to check: 0-3 |

## Return Values

```
|b00| XY-coordinates (1), #CHKXY
|b01| Z-coordinate (2), #CHKZ
|b02|
|b03|
|b04| Rotation (16), #CHKR
|b05| Magnification XY (32), #CHKS
|b06| Display color (64), #CHKC
|b07| Variable (128), #CHKV
A target is assigned for each bit (If 0 is assigned for all bits, animation is being stopped)
```

## Examples

```sb3
ST=BGCHK(0)
'|b00|#CHKXY
'|b01|#CHKZ
'|b04|#CHKR
'|b05|#CHKS
'|b06|#CHKC
'|b07|#CHKV
```
