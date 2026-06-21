---
title: BGOFS
slug: docs-ptc-bgofs
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgofs
content_id: 19629
created: 2023-05-14
scraped: 2026-06-21
---

# BGOFS

Scroll a background layer.

## Syntax

```sbsyntax
BGOFS layer, x, y {, time}
```

| Input | Description |
| --- | --- |
| `layer` | Layer to write tile to. 0 is the foreground, 1 is the background. |
| `x` | x-coordinate to scroll to (in pixels) |
| `y` | y-coordinate to scroll to (in pixels) |
| `time` | Time to scroll for (in frames) |

Scrolls the background layer `layer` to the given coordinates (`x`,`y`). If `time` is omitted or zero, this change is instant - otherwise, the layer is scrolled smoothly from its current position to the new offset over `time` frames.

## Examples

```sb
'place a gray tile at (1,1)
BGPUT 0,1,1,1
'scroll BG layer to (8,8)
'This moves the gray tile in the upper left corner
BGOFS 0,8,8
```

```
'place a red tile at (15,23)
BGPUT 0,15,23,2
'scroll layer smoothly to (0,184) over one second (60 frames)
BGOFS 0,0,184,60
```

## Notes

The background screen is repeated every 64 tiles in both `x` and `y` - if you scroll past the end, you will see the beginning of the layer again.

Despite the repeating of the background, the position is still stored in its entirety, so scrolling further than necessary can be used to move the background layer faster:

```
' Fill a pink square
BGFILL 0,0,0,3,3,3
' Scroll to x=1024 over one second
BGOFS 0,1024,0,60
WAIT 60
BGOFS 0,0,0 'reset position
' Scroll to x=2048 over one second
BGOFS 0,2048,0,60
```

## Errors

| Action | Error |
| --- | --- |
| Zero to two arguments are specified | Missing operand |
| Five or more arguments are specified | Syntax error |
| A value not 0 or 1 is passed for `layer` | Out of range |
| A value less than zero is passed for `time` | Out of range |
| A string value is passed in place of a numeric argument | Type Mismatch |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGPAGE`](https://smilebasicsource.com/forum/thread/docs-ptc-bgpage)
