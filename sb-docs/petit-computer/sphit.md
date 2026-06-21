---
title: SPHIT
slug: docs-ptc-sphit
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-sphit
content_id: 19740
created: 2024-11-30
scraped: 2026-06-21
---

# SPHIT

Check for collisions between a selected sprite and others.

## Syntax

```sbsyntax
hit = SPHIT(id, { start })
```

| Input | Description |
| --- | --- |
| `id` | Id of sprite to check collision for. |
| `start` | Starting id to check collisions from. Optional |

| Output | Description |
| --- | --- |
| `hit` | Indicates if there was a collision detected. |

`SPHIT` checks for collisions between one sprite and every other active sprite. If `start` is specified, then `SPHIT` checks for collisions between sprite `id` and active sprites starting from id `start` and checking up until 99, excluding `id`. A sprite will not collide with itself.

When a collision occurs, the value returned from `SPHIT` will be `TRUE`, and this function also sets the system variables `SPHITNO`, `SPHITX`, `SPHITY`, and `SPHITT`. If no collision occurs, `SPHIT` instead returns false.

The hitboxes and masks used for sprite collision detection can be modified by [`SPCOL`](https://smilebasicsource.com/forum/thread/docs-ptc-spcol) and [`SPCOLVEC`](https://smilebasicsource.com/forum/thread/docs-ptc-spcolvec).

## Examples

```sb
' Create two sprites and check for their collision.
SPSET 0,64,3,0,0,0 ' Create boy sprite
SPSET 1,96,2,0,0,0 ' Create witch sprite
' Check for sprites colliding with the boy
HIT=SPHIT(0)
' If there is a collision, print the colliding sprite's id.
IF HIT THEN PRINT SPHITNO
' In this example, this would print 1.
```

```sb
' Create two sprites and check for their collision.
SPSET 0,64,3,0,0,0 ' Create boy sprite
SPSET 1,96,2,0,0,0 ' Create witch sprite
SPOFS 1,64,0 ' Move witch over 64 pixels
' Check for sprites colliding with the witch
HIT=SPHIT(1)
' If there is a collision, print the colliding sprite's id.
IF HIT THEN PRINT SPHITNO
' In this example, nothing will print, because HIT=0
```

```sb
' Create three sprites and check for their collision.
SPSET 0,96,3,0,0,0 ' Create witch sprite
SPSET 1,64,2,0,0,0 ' Create first boy sprite
SPSET 2,68,4,0,0,0 ' Create second boy sprite
' Check for sprites colliding with the witch, starting with sprite ID 2
HIT=SPHIT(0,2)
' If there is a collision, print the colliding sprite's id.
IF HIT THEN PRINT SPHITNO
' In this example, HIT=TRUE and 2 is printed, since sprite ID 1 was skipped.
```

## Notes

All arguments are rounded down.

`SPHIT` checks for collision with any other sprite. To check for collision between two specific sprites, use `SPHITSP`.

## Errors

| Action | Error |
| --- | --- |
| Less than one argument is passed | Missing operand |
| More than two arguments are passed | Missing operand |
| `id` is less than zero or greater than 99 | Out of range |
| `start` is less than zero or greater than 99 | Out of range |
| The sprite `id` does not exist | Illegal function call |
| A string argument is passed | Type Mismatch |

## See Also

- [`SPCOL`](https://smilebasicsource.com/forum/thread/docs-ptc-spcol)
- [`SPCOLVEC`](https://smilebasicsource.com/forum/thread/docs-ptc-spcolvec)
- [`SPHITSP`](https://smilebasicsource.com/forum/thread/docs-ptc-sphitsp)
- [`SPHITRC`](https://smilebasicsource.com/forum/thread/docs-ptc-sphitrc)
- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
