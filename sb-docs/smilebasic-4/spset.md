---
title: SPSET
slug: docs-sb4-spset
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-spset
content_id: 19530
created: 2020-11-30
scraped: 2026-06-21
---

# SPSET

Create a sprite.

If a definition template ID is specified, then the sprite's UV, width, height, display attributes, and home coordinate are set based on the definition template. Otherwise, the sprite's display properties are specified manually as arguments to the function.

## Syntax (1)

This form creates a sprite using `spriteID%` as its ID.

```sbsyntax
SPSET spriteID%, definitionID% {, showFlag% }
SPSET spriteID%, u%, v%, width%, height% {, attribute% {, showFlag% }}
```

| Input | Description |
| --- | --- |
| `spriteID%` | The sprite ID to use. 0-4095. |
| `definitionID%` | The definition template to use for this sprite. 0-8191. |
| `u%` | Graphic page coordinates to use for the sprite image. |
| `v%` | Graphic page coordinates to use for the sprite image. |
| `width%` | Width/height of the sprite (and its image) in pixels. |
| `height%` | Width/height of the sprite (and its image) in pixels. |
| `attribute%` | A bitset specifying display attributes. Optional, 0 by default.<br>Bit — Description<br>0 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>1 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>2 — Flip image horizontally. `#A_REVH`<br>3 — Flip image vertically. `#A_REVV`<br>4 — Use Add mode blending. `#A_ADD` |
| `showFlag%` | Set whether or not to display the sprite when created. Optional, `#TRUE` by default. |

## Syntax (2)

This form searches a given range (by default, the range of all valid sprite IDs) for the first ID that is not in use, and then creates a sprite using that ID. This is especially useful for games that use lots of temporary sprites, such as item drops, or particle systems. If there are no free sprite IDs, no sprite is created.

```sbsyntax
SPSET { startID%, endID%, } definitionID% {, showFlag% } OUT spriteID%
SPSET { startID%, endID%, } u%, v%, width%, height% {, attribute% {, showFlag% }} OUT spriteID%
```

| Input | Description |
| --- | --- |
| `startID%` | The range to search for free sprite IDs. Optional, default 0-4095.<br>If `startID%` is greater than `endID%`, the range is searched in reverse order. |
| `endID%` | The range to search for free sprite IDs. Optional, default 0-4095.<br>If `startID%` is greater than `endID%`, the range is searched in reverse order. |
| `definitionID%` | The definition template to use for this sprite. 0-8191. |
| `u%` | Graphic page coordinates to use for the sprite image. |
| `v%` | Graphic page coordinates to use for the sprite image. |
| `width%` | Width/height of the sprite (and its image) in pixels. |
| `height%` | Width/height of the sprite (and its image) in pixels. |
| `attribute%` | A bitset specifying display attributes. Optional, 0 by default.<br>Bit — Description<br>0 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>1 — Rotation of the image, in 90 degree steps. `#A_ROT0`, `#A_ROT90`, `#A_ROT180`, `#A_ROT270`<br>2 — Flip image horizontally. `#A_REVH`<br>3 — Flip image vertically. `#A_REVV`<br>4 — Use Add mode blending. `#A_ADD` |
| `showFlag%` | Set whether or not to display the sprite when created. Optional, `#TRUE` by default. |

| Output | Description |
| --- | --- |
| `spriteID%` | The ID used for this sprite (0-4095).<br>If no free sprite IDs are available in the given range, -1 is returned. |

## Examples

```sb4
'set sprite 0 to a strawberry
SPSET 0,0
```

```sb4
'make a 100x100 sprite using the top left corner of GRP4
SPSET 1,0,0,100,100
```

```sb4
'create a bunch of oranges
REPEAT
 I%=SPSET(100,200,2)
 PRINT I%
UNTIL I%==-1
```
