---
title: BGCLR
slug: docs-ptc-bgclr
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgclr
content_id: 19626
created: 2023-05-11
scraped: 2026-06-21
---

# BGCLR

Clear a background layer.

## Syntax

```sbsyntax
BGCLR { layer }
```

| Input | Description |
| --- | --- |
| `layer` | BG layer to clear |

Clears a background layer, filling the layer with tile data 0. If layer is not specified, both layers are cleared. `BGCLR` only clears the current screen. `layer` is rounded down.

## Examples

```sb
' clear both background layers
BGCLR
```

## Errors

| Action | Error |
| --- | --- |
| Provide a string argument | Type Mismatch |
| Layer is not zero or one | Out of range |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGFILL`](https://smilebasicsource.com/forum/thread/docs-ptc-bgfill)
