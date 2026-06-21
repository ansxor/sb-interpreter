---
title: SPHITSP
slug: docs-ptc-sphitsp
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-sphitsp
content_id: 19741
created: 2024-11-30
scraped: 2026-06-21
---

# SPHITSP

Checks for collisions between two selected sprites.

## Syntax

```sbsyntax
hit = SPHITSP(first, second)
```

| Input | Description |
| --- | --- |
| `first` | ID of first sprite to check collision between |
| `second` | ID of second sprite to check collision between |

| Output | Description |
| --- | --- |
| `hit` | Indicates if there was a collision detected |

`SPHITSP` checks for collisions between the two sprites with ids `first` and `second`.

When a collision occurs, the value returned from `SPHITSP` will be `TRUE`, and this function also sets the system variables `SPHITX`, `SPHITY`, and `SPHITT`. If no collision occurs, `SPHITSP` instead returns `FALSE`.
`SPHITSP` does not cause `SPHITNO` to be set, unlike `SPHIT`.

The hitboxes used for sprite collision detection can be modified by [`SPCOL`](https://smilebasicsource.com/forum/thread/docs-ptc-spcol) and [`SPCOLVEC`](https://smilebasicsource.com/forum/thread/docs-ptc-spcolvec).

## Examples

```sb
' Create two sprites and check for their collision.
SPSET 0,64,3,0,0,0 ' Create boy sprite
SPSET 1,96,2,0,0,0 ' Create witch sprite
' Check for collision between the boy and the witch
HIT=SPHITSP(0,1)
PRINT HIT
' Prints 1 (TRUE)
```

```sb
' Create two sprites and check for their collision.
SPSET 0,64,3,0,0,0 ' Create boy sprite
SPSET 1,96,2,0,0,0 ' Create witch sprite
SPOFS 1,64,0 ' Move witch over 64 pixels
' Check for collision between the boy and the witch
HIT=SPHITSP(0,1)
PRINT HIT
' Prints 0 (FALSE)
```

## Notes

All arguments are rounded down.

## Errors

| Action | Error |
| --- | --- |
| Less than two arguments are passed | Syntax error |
| More than three arguments are passed | Missing operand |
| `first` is greater than 99 or less than zero | Out of range |
| `second` is greater than 99 or less than zero | Out of range |
| The sprite `first` does not exist | Illegal function call |
| The sprite `second` does not exist | Illegal function call |
| A string argument is passed | Type Mismatch |

## See Also

- [`SPCOL`](https://smilebasicsource.com/forum/thread/docs-ptc-spcol)
- [`SPCOLVEC`](https://smilebasicsource.com/forum/thread/docs-ptc-spcolvec)
- [`SPHIT`](https://smilebasicsource.com/forum/thread/docs-ptc-sphit)
- [`SPHITRC`](https://smilebasicsource.com/forum/thread/docs-ptc-sphitrc)
- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
