---
title: SPPAGE
slug: docs-sb3-sppage
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPPAGE

> **Category:** Sprites

## SPPAGE (1)

Sets a graphic page to assign to sprites

### Format

```sb3
SPPAGE Graphic page
```

### Arguments

| Argument | Description |
| --- | --- |
| `Graphic page` | 0-5 (GRP0-GRP5) By default, the page for sprites is 4 (GRP4) |

### Examples

```sb3
SPPAGE 4
```

## SPPAGE (2)

Gets the graphic page that has been assigned to sprites

### Format

```sb3
Variable=SPPAGE()
```

### Return Values

```
Graphic page number (0-5)
```

### Examples

```sb3
P=SPPAGE()
```
