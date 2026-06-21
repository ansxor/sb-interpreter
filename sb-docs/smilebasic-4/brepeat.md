---
title: BREPEAT
slug: docs-sb4-brepeat
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-brepeat
content_id: 19520
created: 2020-05-16
scraped: 2026-06-21
---

# BREPEAT

Set/get the repeat settings of a button. This affects mode 1 (Pressed or Repeat) of `BUTTON`. `BREPEAT` settings affect all controllers.

## Syntax

```sbsyntax
BREPEAT id%, repeatDelay%, repeatInterval%
BREPEAT id% OUT repeatDelay%, repeatInterval%
```

| Input | Description |
| --- | --- |
| `id%` | ID of the target button. |
| `repeatDelay%` | Number of frames the button must be held for before starting repeat. |
| `repeatInterval%` | Number of frames between each repeat.<br>Pass 0 to disable repeat on this button. |

| Output | Description |
| --- | --- |
| `repeatDelay%` | Number of frames the button must be held for before starting repeat. |
| `repeatInterval%` | Number of frames between each repeat.<br>If 0, repeat is disabled on this button. |

## Examples

```sb4
'set A to repeat every frame after being held for 15
BREPEAT #B_A,15,1
```

## Button IDs

This table lists the button IDs and constants.

| Name | Value | Description |
| --- | --- | --- |
| `#B_RUP`, `#B_X` | 0 | Top face button / X |
| `#B_RDOWN`, `#B_B` | 1 | Bottom face button / B |
| `#B_RLEFT`, `#B_Y` | 2 | Left face button / Y |
| `#B_RRIGHT`, `#B_A` | 3 | Right face button / A |
| `#B_LUP` | 4 | D-Pad up |
| `#B_LDOWN` | 5 | D-Pad down |
| `#B_LLEFT` | 6 | D-Pad left |
| `#B_LRIGHT` | 7 | D-Pad right |
| `#B_L1`, `#B_SL` | 8 | L trigger / SL trigger |
| `#B_R1`, `#B_SR` | 9 | R trigger / SR trigger |
| `#B_L2`, `#B_S1` | 10 | ZL trigger / Joy-Con side trigger |
| `#B_R2`, `#B_S2` | 11 | ZR trigger / Joy-Con side Z trigger |
| `#B_LSTICK` | 12 | Left stick click |
| `#B_RSTICK` | 13 | Right stick click |
| `#B_RANY` | 14 | Any right side button |
| `#B_LANY` | 15 | Any left side button |
| `#B_ANY` | 16 | Any button |

## See Also

- `BUTTON`
