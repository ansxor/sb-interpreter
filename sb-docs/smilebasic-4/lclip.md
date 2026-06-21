---
title: LCLIP
slug: docs-sb4-lclip
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-lclip
content_id: 19551
created: 2020-11-02
scraped: 2026-06-21
---

# LCLIP

Set/get/clear a layer's clipping area.

## Syntax

```sbsyntax
LCLIP id% {, startX%, startY%, endX%, endY% }
LCLIP id% OUT startX%, startY%, endX%, endY%
```

| Argument | Description |
| --- | --- |
| `id%` | The ID of the target layer. |
| `startX%`, `startY%` | The upper-left coordinate of the clipping rectangle. |
| `endX%`, `endY%` | The lower-right coordinate of the clipping rectangle. |

If the coordinates are omitted, the clipping area is reset.

## Examples

```sb4
'set the clipping area of layer 0
LCLIP 0,100,100,200,200
```

```sb4
'reset the clipping area
LCLIP 0
```

```sb4
'get the clipping area
LCLIP 0 OUT X0,Y0,X1,Y1
```
