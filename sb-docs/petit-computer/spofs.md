---
title: SPOFS
slug: docs-ptc-spofs
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spofs
content_id: 19641
created: 2023-05-31
scraped: 2026-06-21
---

# SPOFS

Change the position of a sprite.

## Syntax

```sbsyntax
SPOFS id, x, y {, time}
```

| Input | Description |
| --- | --- |
| `id` | ID of sprite to move |
| `x` | New x-coordinate |
| `y` | New y-coordinate |
| `time` | Optional interpolation time in frames, zero if not specified |

Moves the sprite referenced by `id` to the specified `x`,`y` coordinates. If time is nonzero, the sprite moves smoothly from the current position to the new position `x`,`y` over `time` frames.

## Examples

```sb
'Create sprite (right arrow)
SPSET 0,0,0,0,0,0
'Move sprite to center of screen
SPOFS 0,112,88
```

```sb
'Create sprite (right arrow)
SPSET 0,0,0,0,0,0
'Move sprite to lower right corner over two seconds
SPOFS 0,240,176,120
```

## Notes

All arguments are rounded down.

The range of `x` and `y` positions is limited to a signed 16 bit number, with range [-32768,32767]. Values outside of this range will be limited to this range.

```sb
'Create sprite (down arrow)
SPSET 0,1,0,0,0,0
'"Move" sprite to upper left corner
SPOFS 0,65536,-65536
'Get and print stored coordinates
SPREAD(0),X,Y
'Prints "0   0"
PRINT X,Y
```

If a sprite is being moved over time and the movement is interrupted by another interpolated `SPOFS`, the start position is the currently interpolated position.

```sb
'Create sprite (up arrow)
SPSET 0,3,0,0,0,0
'Move to lower right corner over two seconds
SPOFS 0,240,176,120
'Interrupt after one second
WAIT 60
'Move to upper right corner instead
SPOFS 0,240,0,120
```

## Errors

| Action | Error |
| --- | --- |
| Less than three arguments are specified | Missing operand |
| More than four arguments are specified | Syntax error |
| `id` is less than zero or greater than 99 | Out of range |
| The sprite `id` does not exist | Illegal function call |
| A value less than zero is passed for `time` | Out of range |
| A string is passed in place of any argument | Type Mismatch |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPHOME`](https://smilebasicsource.com/forum/thread/docs-ptc-sphome)
- [`SPREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-spread)
