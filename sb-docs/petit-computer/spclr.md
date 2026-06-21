---
title: SPCLR
slug: docs-ptc-spclr
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spclr
content_id: 19637
created: 2023-05-25
scraped: 2026-06-21
---

# SPCLR

Clear a sprite or all sprites.

## Syntax

```sbsyntax
SPCLR id
```

| Input | Description |
| --- | --- |
| `id` | Sprite ID to clear. |

Clears the sprite specified by `id`. If `id` is omitted, clears all sprites on the current sprite screen.

## Examples

```sb
'create a sprite with id 5
SPSET 5,64,0,0,0,0
'clear sprite 5
SPCLR 5
```

```sb
' Create several sprites and place them randomly on screen
FOR I=0 TO 9
 SPSET I,RND(128),I,0,0,0
 SPOFS I,RND(240),RND(176)
NEXT
WAIT 60
' Clear all sprites
SPCLR
```

## Notes

`id` is rounded down.

Attempting to `SPCLR` a sprite that is already cleared will have no effect and no error is thrown.

For some reason, you can add a trailing comma to `SPCLR` if an argument is passed. This causes the first argument to be ignored, even if it's a string, and `SPCLR` will clear all sprites on the current screen.

```sb
' Acts like SPCLR with no arguments
SPCLR "Why",
```

## Errors

| Action | Error |
| --- | --- |
| A value less than zero or greater than 99 is passed for `id` | Out of range |
| A string argument is passed | Type Mismatch |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-spset)
- [`BGCLR`](https://smilebasicsource.com/forum/thread/docs-ptc-bgclr)
