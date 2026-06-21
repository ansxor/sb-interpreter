---
title: SPCHR
slug: docs-ptc-spchr
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-spchr
content_id: 19647
created: 2023-06-04
scraped: 2026-06-21
---

# SPCHR

Change the appearance of a sprite.

## Syntax

```sbsyntax
SPCHR id, chr {, pal, h, v, prio}
```

| Input | Description |
| --- | --- |
| `id` | Sprite ID |
| `chr` | New character code |
| `pal` | New sprite color palette. Optional |
| `h` | Set sprite horizontal flip state. Optional |
| `v` | Set sprite vertical flip state. Optional |
| `prio` | Set graphics draw priority. Optional |

Sets the new sprite character `chr` for sprite `id`. If specified, can also set new values of `pal`,`h`,`v`, and `prio` simultaneously. Either all optional arguments or none must be specified.

## Examples

```sb
'Create boy sprite
SPSET 0,64,2,0,0,0
'Change to witch sprite (keeps palette and other values same)
SPCHR 0,96
```

```sb
'Create boy sprite
SPSET 0,64,2,0,0,0
'Change to red skeleton sprite (with horizontal flip and new draw priority)
SPCHR 0,128,4,1,0,1
```

## Notes

All arguments are rounded down.

The size of the sprite cannot be modified using `SPCHR`. To change the sprite size, `SPSET` must be used, but this will reset several other sprite properties as well. See [`SPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-spset) for more info.

## Errors

| Action | Error |
| --- | --- |
| Zero or one arguments are passed | Missing operand |
| Between three and five arguments are passed | Missing operand |
| Seven or more arguments are passed | Syntax error |
| `id` does not correspond to an active sprite | Illegal function call |
| `id` is less than zero or greater than 99 | Out of range |
| `chr` is less than zero or greater than 511 on the upper screen | Out of range |
| `chr` is less than zero or greater than 117 on the lower screen | Out of range |
| `pal` is less than zero or greater than 15 | Out of range |
| `h` is not zero or one | Out of range |
| `v` is not zero or one | Out of range |
| `prio` is less than zero or greater than three | Out of range |
| A string is passed for any argument | Type Mismatch |

## See also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`SPSET`](https://smilebasicsource.com/forum/thread/docs-ptc-spset)
- [`SPANIM`](https://smilebasicsource.com/forum/thread/docs-ptc-spanim)
