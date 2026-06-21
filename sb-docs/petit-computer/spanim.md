---
title: SPANIM
slug: docs-ptc-spanim
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spanim
content_id: 19738
created: 2024-09-22
scraped: 2026-06-21
---

# SPANIM

Animate a sprite.

## Syntax

```sbsyntax
SPANIM id, chr_count, frame_time {, loop_count}
```

| Input | Description |
| --- | --- |
| `id` | Sprite ID |
| `chr_count` | Number of different sprites to change to |
| `frame_time` | Number of frames to linger on a given sprite |
| `loop_count` | Number of times to repeat animation. Optional |

Animates sprite `id` by cycling through `chr_count` sprites, waiting `frame_time` for each sprite before changing. If specified, repeats the cycle `loop_count` times, or repeats forever if `loop_count` is zero or omitted.

## Examples

```
' Create a boy sprite facing the camera
SPSET 0,68,2,0,0,0
' Aniamte the sprite so it appears to walk
' The sprite takes one second per cycle (4 sprites * 15 frames/sprite)
SPANIM 0,4,15
```

## Notes

All arguments rounded down.

Note that the `SPCHR`/`SPSET` character selected is independent of the animation. If you use `SPREAD` to get the current character, it will always return whatever character the sprite was last set to or created with. As such you can not determine the current animation frame by `SPREAD`.

If you use `SPCHR` on an animated sprite, the animation settings are kept, but the shift in base character leads to the animation changing. Example:

```
' Create a boy sprite facing the camera
SPSET 0,68,2,0,0,0
' Aniamte the sprite so it appears to walk
' The sprite takes one second per cycle (4 sprites * 15 frames/sprite)
SPANIM 0,4,15
' Wait for one cycle
WAIT 60
' Change the boy to face right
SPCHR 0,64
' Now the boy appears walking to the right
```

## Errors

| Action | Error |
| --- | --- |
| `id` does not correspond to an active sprite | Illegal function call |
| `id` is less than zero or greater than 99 | Out of range |
| `chr_count` is less than one | Out of range |
| `frame_time` is less than zero | Out of range |
| `loop_count` is less than zero | Out of range |
| A string argument is passed | Type Mismatch |
| Less than three arguments are passed | Missing operand |
| More than four arguments are passed | Syntax error |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-spofs)
- [`SPCHR`](https://smilebasicsource.com/forum/thread/docs-ptc-spchr)
- [`SPCHK`](https://smilebasicsource.com/forum/thread/docs-ptc-spchk)
