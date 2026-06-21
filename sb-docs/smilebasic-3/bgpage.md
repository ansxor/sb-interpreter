---
title: BGPAGE
slug: docs-sb3-bgpage
system: SmileBASIC 3
type: command
category: BG
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# BGPAGE

> **Category:** BG

## BGPAGE (1)

Sets a graphic page to assign to BG

### Format

```sb3
BGPAGE Graphic page
```

### Arguments

| Argument | Description |
| --- | --- |
| `Graphic page` | 0-5 (GRP0-GRP5) By default, the graphic page for BG is 5 (GRP5) |

### Examples

```sb3
BGPAGE 5
```

## BGPAGE (2)

Gets the graphic page that has been assigned to BG

### Format

```sb3
Variable=BGPAGE()
```

### Return Values

```
Graphic page number (0-5)
```

### Examples

```sb3
P=BGPAGE()
```
