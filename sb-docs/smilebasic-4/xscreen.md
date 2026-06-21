---
title: XSCREEN
slug: docs-sb4-xscreen
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-xscreen
content_id: 19527
created: 2020-05-16
scraped: 2026-06-21
---

# XSCREEN

Set/get the display mode.

## Syntax

```sbsyntax
XSCREEN width%, height% {, sampleFactor% {, interpolation% {, aspect# }}}
XSCREEN OUT width%, height% {, sampleFactor% {, interpolation% {, aspect# }}}
```

| Parameter | Description |
| --- | --- |
| `width%` | Width and height of the display mode, in pixels.<br>Must be in multiples of 4. `width%` must be between 128 and 1280. `height%` must be between 128 and 720. |
| `height%` | Width and height of the display mode, in pixels.<br>Must be in multiples of 4. `width%` must be between 128 and 1280. `height%` must be between 128 and 720. |
| `sampleFactor%` | Factor to use for supersampling. (optional, default 1.)<br>`width% * sampleFactor%` must not be greater than 1280 and `height% * sampleFactor%` must not be greater than 720. |
| `interpolation%` | Interpolation mode used for scaling the screen (optional.)<br><br>0 — Bilinear (default)<br>1 — Smart Nearest-Neighbor<br>2 — Nearest-Neighbor |
| `aspect#` | Display aspect ratio to use. (optional, default `width% / height%`) |

## Examples

```sb4
'set the display to 720p
XSCREEN 1280,720
```

```sb4
'check the current display setting
XSCREEN OUT W%,H%
PRINT W%,H%
```

## Notes

### Additional Effects

`XSCREEN` also automatically adjusts the sizing of the console text screen and graphic page sprite.
