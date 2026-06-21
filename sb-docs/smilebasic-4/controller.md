---
title: CONTROLLER
slug: docs-sb4-controller
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-controller
content_id: 19519
created: 2020-05-16
scraped: 2026-06-21
---

# CONTROLLER

Check the type of connected controllers.

## Syntax

```sbsyntax
type% = CONTROLLER(id%)
CONTROLLER id% OUT type% {, mainColor%, subColor% }
CONTROLLER id% OUT type% {, leftMainColor%, leftSubColor%, rightMainColor%, rightSubColor% }
```

| Input | Description |
| --- | --- |
| `id%` | ID of the controller to check. 0-4; 0 is the "default" controller (see notes). |

| Output | Description |
| --- | --- |
| `type%` | The type of controller connected: |

| Number | Description |
| --- | --- |
| 0 | Not connected |
| 1 | Handheld mode |
| 2 | Pro Controller |
| 3 | Dual Joy-Con / Joy-Con Grip |
| 4 | Left Joy-Con |
| 5 | Right Joy-Con |

| `mainColor%` | The primary color of the controller. |
| `subColor%` | The secondary color of the controller. |
| `leftMainColor%` | The primary color of the left half of the controller. |
| `leftSubColor%` | The secondary color of the left half of the controller. |
| `rightMainColor%` | The primary color of the right half of the controller. |
| `rightSubColor%` | The secondary color of the right half of the controller. |

## Examples

```sb4
CONTROLLER 1 OUT T%,M%,S%
PRINT T%
BACKCOLOR M%
```

## Notes

### `XTCTRLSTYLE`

The number and types of controllers that can be connected is affected by `XCTRLSTYLE`. Use that function to prompt for controller connection if necessary.

### Default Controller

Controller ID 0 is used as the "default" controller. This controller ID is always treated as a Handheld controller and combines all of the button/stick inputs into it. It is primarily useful for handling any control style in single-user programs such as tools.

### Handheld Mode

If the Switch is in handheld mode (which is always true for Switch Lite) controller ID 1 will always be the Handheld controller. Note that if another controller is connected, this is interpreted as Tabletop mode, even if the Joy-Cons are still connected. As a result, the connected Joy-Cons are not assigned a controller ID, but their input is still recognized by controller ID 0.

### Controller Color

The controller color return values are used to check the color of the controller itself, e.g. if your Joy-Con is red or blue. These can be used , for example, to customize the graphics in a game based on the color of the connected controller. If a controller is not connected, the values always return black. For Handheld (1) and Dual Joy-Con (3) type controllers, the left and right side primary and secondary colors are returned separately using the four return values. For all other controllers, the two return values are used for primary and secondary color. When accepting four return values for these controllers, the other two color values are always black.

## See Also

- [`BUTTON`](https://smilebasicsource.com/forum/thread/1335)
- `STICK`
- `XCTRLSTYLE`
