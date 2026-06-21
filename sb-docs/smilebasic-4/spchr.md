---
title: SPCHR
slug: docs-sb4-spchr
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spchr
content_id: 19539
created: 2020-12-14
scraped: 2026-06-21
---

# SPCHR

Set/get the image display properties of the sprite.

If a definition template ID is specified, then the sprite's UV, width, height, display attributes, and home coordinate are set based on the definition template. Otherwise, the sprite's display properties (aside from home coordinate) are specified manually as arguments to the function.

## Set

Set the properties of the sprite, using either a template ID or setting them manually.

```sbsyntax
SPCHR spriteID%, definitionID%
SPCHR spriteID%, u%, v%, width%, height% {, attribute% }
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. 0-4095. |
| `definitionID%` | The definition template to use for this sprite. 0-8191. |
| `u%` | Graphic page coordinates to use for the sprite image. |
| `v%` | Graphic page coordinates to use for the sprite image. |
| `width%` | Width/height of the sprite (and its image) in pixels. |
| `height%` | Width/height of the sprite (and its image) in pixels. |
| `attribute%` | A bitset specifying display attributes. Optional, 0 by default.<br>Bit — Description<br>0 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>1 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>2 — Flip image horizontally. `#A_REVH`<br>3 — Flip image vertically. `#A_REVV`<br>4 — Use Add mode blending. `#A_ADD` |

## Get

Get the sprite's display properties.

```sbsyntax
SPCHR spriteID% OUT definitionID%
SPCHR spriteID% OUT u%, v% {, width%, height% {, attribute% }}
```

| Input | Description |
| --- | --- |
| `spriteID%` | ID of the target sprite. 0-4095. |

| Output | Description |
| --- | --- |
| `definitionID%` | The definition template used for this sprite. 0-8191.<br>If this sprite was not set with a definition template, this value is 0 anyway. |
| `u%` | Graphic page coordinates used for the sprite image. |
| `v%` | Graphic page coordinates used for the sprite image. |
| `width%` | Width/height of the sprite (and its image) in pixels. |
| `height%` | Width/height of the sprite (and its image) in pixels. |
| `attribute%` | A bitset specifying display attributes. Optional, 0 by default.<br>Bit — Description<br>0 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>1 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>2 — Flip image horizontally. `#A_REVH`<br>3 — Flip image vertically. `#A_REVV`<br>4 — Use Add mode blending. `#A_ADD` |
