---
title: BGCHK
slug: docs-ptc-bgchk
system: Petit Computer
type: command
source: https://smilebasicsource.com/forum/thread/docs-ptc-bgchk
content_id: 19634
created: 2023-05-19
scraped: 2026-06-21
---

# BGCHK

Check if a background layer is being scrolled.

## Syntax

```sbsyntax
scroll = BGCHK(layer)
```

| Input | Description |
| --- | --- |
| `layer` | Layer to check status of |

| Output | Description |
| --- | --- |
| `scroll` | 1 if layer is currently moving, 0 otherwise |

Checks if `layer` is currently being scrolled by a previous `BGOFS` command. 1 indicates that the layer is currently animating, 0 indicates that there is no movement.

## Examples

```sb
'Layer 0 is not moving yet
'0 is printed
PRINT BGCHK(0)

'Make layer 0 move for 30 frames
BGOFS 0,30,30,30
'1 is printed
PRINT BGCHK(0)

'Wait for movement to end
WAIT 30
'0 is printed
PRINT BGCHK(0)
```

## Notes

All arguments are rounded down.

For some reason, specifying an invalid layer causes an `Overflow` error instead of an `Out of range` error.

Note that in v1 of Petit Computer, this command did not exist.

## Errors

| Action | Error |
| --- | --- |
| Zero arguments are passed | Syntax error |
| Two or more arguments are passed | Missing operand |
| A string is passed | Type Mismatch |
| A value not 0 or 1 is passed for `layer` | Overflow |

## See Also

- [Background overview](https://smilebasicsource.com/forum/thread/docs-ptc-background)
- [`BGOFS`](https://smilebasicsource.com/forum/thread/docs-ptc-bgofs)
