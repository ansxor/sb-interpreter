---
title: GPAGE
slug: docs-sb3-gpage
system: SmileBASIC 3
type: command
category: Graphics
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# GPAGE

> **Category:** Graphics

## GPAGE (1)

Specifies a page for graphic display and a page for manipulation

### Format

```sb3
GPAGE Display page, Manipulation page
```

### Arguments

| Argument | Description |
| --- | --- |
| `Display page` | 0-5: GRP0-GRP5 |
| `Manipulation page` | 0-5: GRP0-GRP5<br>* By default, GRP4 contains sprites and GRP5 contains BG images. |

### Examples

```sb3
GPAGE 0,0
```

## GPAGE (2)

Gets information on the graphic page currently set

### Format

```sb3
GPAGE OUT VP,WP
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `VP` | Page number for display (0-5) |
| `WP` | Page number for manipulation (0-5) |

### Examples

```sb3
GPAGE OUT WP,GP
```
