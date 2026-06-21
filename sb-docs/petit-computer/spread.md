---
title: SPREAD
slug: docs-ptc-spread
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spread
content_id: 19653
created: 2023-06-18
scraped: 2026-06-21
---

# SPREAD

Read sprite attributes.

## Syntax

```sbsyntax
SPREAD(id), x {, y {, angle {, scale {, chr}}}}
```

| Input | Description |
| --- | --- |
| id | Sprite id |

| Output | Description |
| --- | --- |
| x | Variable to store current x-coordinate |
| y | Variable to store current y-coordinate |
| angle | Variable to store current sprite rotation angle |
| scale | Variable to store current sprite scale |
| chr | Variable to store current sprite character |

Reads various attributes from a sprite, including position, angle, scale, and character. Useful to check the exact state of interpolated values.

## Examples

```sb
'Create sprite (boy)
SPSET 0,64,2,0,0,0
'Move sprite to 120,80
SPOFS 0,120,80
'Read and display sprite coordinates
SPREAD(0),X,Y
'120 80
PRINT X,Y
```

```sb
'Create sprite (witch)
SPSET 0,96,2,0,0,0
'Move, scale, and rotate sprite over time
SPOFS 0,200,120,120
SPANGLE 0,180,150
SPSCALE 0,200,180
'Read values partway through changes
WAIT 90
SPREAD(0),X,Y,A,S,C
PRINT X,Y,A,S,C
'150 90  107.996 150 96
```

## Notes

`id` is rounded down.

The values for `x`, `y`, and `chr` will be integers, while `angle` and `scale` can have decimal components.

There is no way to read the current palette of the sprite.

## Errors

| Action | Error |
| --- | --- |
| `id` is not specified | Syntax error |
| More than two arguments are specified within the parenthesis | Missing operand |
| No variables are provided as output | Missing operand |
| More than five variables are provided as output | Illegal function call |
| `id` is less than zero or greater than 100 | Out of range |
| `id` does not correspond to an active sprite | Illegal function call |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPCHK`](https://smilebasicsource.com/forum/thread/docs-ptc-spchk)
- [`SPOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-spofs)
- [`SPSCALE`](https://smilebasicsource.com/forum/thread/docs-ptc-spscale)
- [`SPANGLE`](https://smilebasicsource.com/forum/thread/docs-ptc-spangle)
- [`SPANIM`](https://smilebasicsource.com/forum/thread/docs-ptc-spanim)
