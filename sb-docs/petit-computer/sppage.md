---
title: SPPAGE
slug: docs-ptc-sppage
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-sppage
content_id: 19636
created: 2023-05-24
scraped: 2026-06-21
---

# SPPAGE

Set the screen to modify sprites of.

## Syntax

```sbsyntax
SPPAGE screen
```

| Input | Description |
| --- | --- |
| `screen` | Screen to modify. 0 is the top screen, 1 is the bottom screen. |

Sets the current sprite screen. This changes which screen all following sprite commands will affect.

## Examples

```sb
PNLTYPE "OFF" 'so lower screen is visible
' Write something to the lower screen's sprites
SPPAGE 1
SPSET 0,64,0,0,0,0 'boy sprite
```

```sb
' Write something to the upper screen's sprites
SPPAGE 0
SPSET 0,96,0,0,0,0 'witch sprite
```

## Notes

`screen` is rounded down.

To actually see the lower screen sprites, it is necessary to first disable the keyboard with `PNLSTR "OFF"`.

`SPPAGE` also influences other commands that interact with the sprite system's resources. For example, this includes `LOAD`, `SAVE`, the `CHR*` commands, and the `COL*` commands if specifying resource types without a screen.

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are provided | Syntax error |
| `screen` is not zero or one | Out of range |
| A string argument is provided for `screen` | Type Mismatch |

## See Also

- [Sprite overview](https://smilebasicsource.com/forum/thread/docs-ptc-sprite)
- [`BGPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-bgpage)
