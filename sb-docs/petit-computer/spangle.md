---
title: SPANGLE
slug: docs-ptc-spangle
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spangle
content_id: 19703
created: 2024-01-03
scraped: 2026-06-21
---

# SPANGLE

Change the rotation of a sprite.

## Syntax

```sbsyntax
SPANGLE id, angle {, time {, direction}}
```

| Input | Description |
| --- | --- |
| id | Sprite id |
| angle | New sprite angle, in degrees |
| time | Time to change to new angle, in frames |
| direction | Direction of rotation during interpolation |

Changes the rotation of sprite `id` to the given angle `angle`. The sprite is rotated around the `SPHOME` position. If `time` is provided, the sprite rotates clockwise over `time` frames to the new angle. If `direction` is specified, it determines the direction of rotation, with -1 indicating counterclockwise rotation and 1 indicating clockwise rotation.

## Examples

```sb
'Create sprite (right arrow)
SPSET 0,0,0,0,0,0
'Set rotation point to sprite center
SPHOME 0,8,8
'Rotate arrow 90 degrees clockwise
SPANGLE 0,90
'Arrow is now facing down
```

```sb
'Create sprite (right arrow)
SPSET 0,0,0,0,0,0
'Set rotation point to sprite center
SPHOME 0,8,8
'Rotate arrow 90 degrees counterclockwise over 1 second (60 frames)
SPANGLE 0,270,60,-1
'Arrow is now facing up
```

## Notes

All arguments are rounded down.

While any valid number is accepted as an angle, the value will be taken as if the angle was within the range of [0,359]. This means that `SPANGLE 0,0,30` is equivalent to `SPANGLE 0,360,30`, for example.

If a sprite is being rotated over time and the rotation is interrupted by another interpolated `SPANGLE`, the starting angle is the currently interpolated position.

The `direction` argument only impacts the interpolation direction - it does not change the value of the provided angle.

## Errors

| Action | Error |
| --- | --- |
| Fewer than two arguments are passed | Missing operand |
| More than four arguments are passed | Syntax error |
| A string argument is passed | Type Mismatch |
| A value less than zero or greater than 31 is passed for `id` | Out of range |
| `id` does not correspond to an active sprite | Illegal function call |
| A value less than zero is passed for `time` | Out of range |
| A value not equal to 1 or -1 is passed for `direction` | Out of range |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPHOME`](https://smilebasicsource.com/forum/thread/docs-ptc-sphome)
- [`SPREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-spread)
