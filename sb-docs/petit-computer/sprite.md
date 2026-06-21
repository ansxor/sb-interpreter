---
title: Sprite overview
slug: docs-ptc-sprite
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-sprite
content_id: 19642
created: 2023-06-01
scraped: 2026-06-21
---

# Sprite overview

## Summary

Each screen allows a maximum of 100 sprites to be in use. These sprites have a variety of potential operations - they can be created in various sizes, moved across the screen, and placed on different layers. Sprites can also use the built-in sprite collision and sprite animation functions, as well as the sprite variables. Sprites 0-31 support some additional operations, like rotation and scaling.

## Interface

### Commands

| | |
| --- | --- |
| [`SPPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-sppage) | Sets the current screen to modify sprites of |
| [`SPCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-spclr) | Clears one sprite or all sprites on one screen. |
| [`SPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-spset) | Creates a new sprite. |
| [`SPOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-spofs) | Moves a sprite. |
| [`SPHOME`](https://smilebasicsource.com/forum/thread/docs-ptc-sphome) | Sets the origin point of a sprite |
| [`SPCHR`](https://smilebasicsource.com/forum/thread/docs-ptc-spchr) | Changes the appearance of a sprite |
| [`SPANIM`](https://smilebasicsource.com/forum/thread/docs-ptc-spanim) | Animates a sprite. |
| [`SPANGLE`](https://smilebasicsource.com/forum/thread/docs-ptc-spangle) | Rotates a sprite |
| [`SPSCALE`](https://smilebasicsource.com/forum/thread/docs-ptc-spscale) | Scales a sprite |
| [`SPREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-spread) | Reads properties from a sprite |
| [`SPSETV`](https://smilebasicsource.com/forum/thread/docs-ptc-spsetv) | Stores a value to a sprite variable. |
| [`SPCOL`](https://smilebasicsource.com/forum/thread/docs-ptc-spcol) | Sets properties of a sprite for collision. |
| [`SPCOLVEC`](https://smilebasicsource.com/forum/thread/docs-ptc-spcolvec) | Sets a vector to be used during sprite collision. |

### Functions

| | |
| --- | --- |
| [`SPGETV`](https://smilebasicsource.com/forum/thread/docs-ptc-spgetv) | Gets the value from a sprite variable. |
| [`SPCHK`](https://smilebasicsource.com/forum/thread/docs-ptc-spchk) | Checks if the sprite is being animated in various ways. |
| [`SPHIT`](https://smilebasicsource.com/forum/thread/docs-ptc-sphit) | Checks if the sprite has hit any other sprite. |
| [`SPHITSP`](https://smilebasicsource.com/forum/thread/docs-ptc-sphitsp) | Checks if the sprite has hit some specific sprite. |
| [`SPHITRC`](https://smilebasicsource.com/forum/thread/docs-ptc-sphitrc) | Checks if the sprite has hit a rectangular region. |

### System Variables

Under construction:

| | |
| --- | --- |
| `SPHITNO` | Stores the id of the sprite that was hit `SPHIT` |
| `SPHITX` | Stores the x-coordinate of the sprite collision |
| `SPHITY` | Stores the y-coordinate of the sprite collision |
| `SPHITT` | Stores a value between 0 and 1 related to the distance between sprites |

Note: `SPHITX` and `SPHITY` depend the `SPOFS` coordinates of the sprite. `SPHITY` appears to depend on `SPHITX`. `SPHITT` depends on all of the above.

TODO: This needs a better explanation.

### Resources

| | |
| --- | --- |
| `SPU0`-`SPU7` | Resources for upper screen sprite characters |
| `SPS0L`-`SPS1L` | Resources for lower screen sprite characters |
| `COL1U` | Upper screen sprite palettes |
| `COL1L` | Lower screen sprite palettes |

## Additional Information

Sprites are formed from 8x8 16-color characters, similar to the background layers. The size of a sprite determines how many characters are used. Characters are placed in the sprite going from left to right, then top to bottom. The color palette can only be specified for an entire sprite, not individual characters like a background.

For example, here are some sprite sizes and character layouts:

```
16x16   32x16   16x32
01      0123    01
23      4567    23
                45
                67
```
