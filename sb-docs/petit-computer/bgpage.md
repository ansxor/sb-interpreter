---
title: BGPAGE
slug: docs-ptc-bgpage
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgpage
content_id: 19630
created: 2023-05-15
scraped: 2026-06-21
---

# BGPAGE

Set the current background screen to modify.

## Syntax

```sbsyntax
BGPAGE screen
```

| Input | Description |
| --- | --- |
| `screen` | Screen to modify. 0 is the top screen, 1 is the bottom screen. |

Sets the current background screen. This changes which screen all following BG commands will affect.

## Examples

```sb
PNLTYPE "OFF" 'so lower screen BG layers are visible
' Write something to the lower screen's BG layers
BGPAGE 1
BGPUT 0,1,2,3 'pink tile at (1,2)
```

```sb
' Write something to the upper screen's BG layers
BGPAGE 0
BGPUT 0,1,1,2 'red tile at (1,1)
```

## Notes

`screen` is rounded down.

To actually see the lower screen's BG layers, it is necessary to first disable the keyboard with `PNLSTR "OFF"`.

`BGPAGE` also influences other commands that interact with the background system's resources. For example, this includes `LOAD`, `SAVE`, the `CHR*` commands, and the `COL*` commands if specifying resource types without a screen.

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are provided | Syntax error |
| `screen` is not zero or one | Out of range |
| A string argument is provided for `screen` | Type Mismatch |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
