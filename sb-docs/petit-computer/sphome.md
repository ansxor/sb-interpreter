---
title: SPHOME
slug: docs-ptc-sphome
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-sphome
content_id: 19640
created: 2023-05-30
scraped: 2026-06-21
---

# SPHOME

Set the sprite origin point.

## Syntax

```sbsyntax
SPHOME id, x, y
```

| Input | Description |
| --- | --- |
| `id` | Sprite id |
| `x` | x-coordinate for sprite origin |
| `y` | y-coordinate for sprite origin |

Sets the origin point of the sprite. This is the point that `SPOFS` controls the location of, as well as the rotation and scaling center point.

## Examples

```sb
'Create sprite
SPSET 0,64,2,0,0,0
'Sets origin to (8,8) (middle of the sprite)
SPHOME 0,8,8
'Coordinates set the position of the center
'instead of the top left corner of the sprite
SPOFS 0,24,24
```

```sb
'Create sprite
SPSET 0,96,2,0,0,0
'The home point can be placed beyond the sprite's edge
SPHOME 0,0,-24
```

## Notes

All arguments are rounded down.

Usable values for `x` and `y` are limited to the range of [-128,127]. Values outside of this range are converted to `value AND 0xFF`, with bit 7 as the sign bit.

```
'Create sprite
SPSET 0,96,2,0,0,0
'Set sprite home to furthest possible point
SPHOME 0,-128,0
WAIT 60
'The following are equivalent to -128,0
SPHOME 0,128,0
WAIT 60
SPHOME 0,384,0

```

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are passed | Syntax error |
| One or two arguments are passed | Missing operand |
| Four or more arguments are passed | Missing operand |
| A string argument is passed | Type Mismatch |
| A value less than zero or greater than 99 is passed for `id` | Out of range |
| The sprite `id` does not exist | Illegal function call |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-spofs)
