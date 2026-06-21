---
title: SPSETV
slug: docs-ptc-spsetv
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spsetv
content_id: 19737
created: 2024-09-21
scraped: 2026-06-21
---

# SPSETV

Set the value of a sprite variable.

## Syntax

```sbsyntax
SPSETV id, var_id, value
```

| Input | Description |
| --- | --- |
| `id` | Sprite ID |
| `var_id` | Variable ID to set (0-7) |
| `value` | The value to store to the variable |

Stores to sprite `id`'s variable `var_id` the value of `value`. The sprite variables are specific to each sprite, so two sprites with different IDs have different sets of sprite variables. Variables stored can be read using `SPGETV`.

## Examples

```sb
' Create a boy sprite
SPSET 0,64,2,0,0,0
' Store the HP (100) of the character as sprite variable 0
HP=100
SPSETV 0,0,HP
```

## Notes

`id` and `var_id` are rounded down. `value` can be any number.

Sprite variables are all reset to zero when a sprite is cleared with `SPCLR`.

## Errors

| Action | Error |
| --- | --- |
| `id` does not correspond to an active sprite | Illegal function call |
| `id` is less than zero or greater than 99 | Out of range |
| `var_id` is less than zero or greater than seven | Out of range |
| A string is passed for any argument | Type Mismatch |
| A positive number of arguments not equal to three are passed | Missing operand |
| No arguments are passed | Syntax error |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPGETV`](https://smilebasicsource.com/forum/thread/docs-ptc-spgetv)
