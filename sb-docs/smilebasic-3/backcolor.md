---
title: BACKCOLOR
slug: docs-sb3-backcolor
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BACKCOLOR

> **Category:** Screen control

## BACKCOLOR (1)

Specifies a background color

### Format

```sb3
BACKCOLOR Background color code
```

### Arguments

| Argument | Description |
| --- | --- |
| `Background color<br>code` | - Usually specified with the RGB function, e.g., BACKCOLOR RGB(64,128,128)<br>- To specify a numerical value directly, a color code consisting of an 8-bit value for each<br>RGB element should be specified |

### Examples

```sb3
BACKCOLOR RGB(64,128,128)
```

## BACKCOLOR (2)

Specifies the current background color

### Format

```sb3
Variable=BACKCOLOR()
```

### Return Values

Color code of the background color currently set

### Examples

```sb3
C=BACKCOLOR()
```
