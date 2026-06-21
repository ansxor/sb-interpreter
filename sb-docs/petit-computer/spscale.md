---
title: SPSCALE
slug: docs-ptc-spscale
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spscale
content_id: 19652
created: 2023-06-08
scraped: 2026-06-21
---

# SPSCALE

Change the size of a sprite.

## Syntax

```sbsyntax
SPSCALE id, scale {, time}
```

| Input | Description |
| --- | --- |
| id | Sprite id |
| scale | New sprite scale, as a percentage |
| time | Time to change to new scale, in frames |

Changes the scale of sprite `id` to the given size `scale`. This scales up the sprite based on the `SPHOME` position. If `time` is provided, the sprite's scale changes smoothly from the old scale to the new scale `scale`.

## Examples

```sb
'Create sprite (star)
SPSET 0,156,3,0,0,0
'Scale star to 50%
SPSCALE 0,50
```

```sb
'Create sprite (star)
SPSET 0,156,3,0,0,0
'Scale to 200% over 60 frames (1 second)
SPSCALE 0,200,60
```

## Notes

`id` and `time` are rounded down.

`scale` accepts decimal values, as long as they are within the normal range of [0,200]. These values will scale the sprite appropriately.

If a sprite is being scaled over time and the scaling is interrupted by another interpolated `SPSCALE`, the starting scale is the currently interpolated position.

## Errors

| Action | Error |
| --- | --- |
| Fewer than two arguments are passed | Missing operand |
| More than three arguments are passed | Syntax error |
| A string argument is passed | Type Mismatch |
| A value less than zero or greater than 31 is passed for `id` | Out of range |
| `id` does not correspond to an active sprite | Illegal function call |
| A value less than zero or greater than 200 is passed for `scale` | Out of range |
| A value less than zero is passed for `time` | Out of range |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPHOME`](https://smilebasicsource.com/forum/thread/docs-ptc-sphome)
- [`SPREAD`](https://smilebasicsource.com/forum/thread/docs-ptc-spread)
