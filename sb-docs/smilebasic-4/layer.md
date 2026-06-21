---
title: LAYER
slug: docs-sb4-layer
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-layer
content_id: 19549
created: 2020-11-02
scraped: 2026-06-21
---

# LAYER

Set properties on a layer.

## Syntax

```sbsyntax
LAYER id% {, compositeMode% {, multiplyColor% }}
```

| Input | Description |
| --- | --- |
| `id%` | The ID of the layer to configure. |
| `compositeMode%` | The layer's composition mode.<br>Value — Description<br>0 — None (overwrite)<br>1 — Simple (alpha)<br>2 — Add<br>3 — Multiply<br>4 — Screen<br>Optional. If not specified, 0. |
| `multiplyColor%` | The layer's multiply color. The multiply color filter is applied to the layer before composition.<br>Optional. If not specified, `#C_WHITE`. |

## Examples

```sb4
'change the properties of layer 0
'simple composition, red multiply color
LAYER 0,1,#C_RED
```

```sb4
'reset the properties of layer 0
LAYER 0
```

## Notes

### No Getter

There is no getter/`OUT` version of this function, meaning these properties cannot be checked after they are set.
