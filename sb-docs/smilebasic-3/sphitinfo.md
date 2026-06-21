---
title: SPHITINFO
slug: docs-sb3-sphitinfo
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 3
scraped: 2026-06-21
---

# SPHITINFO

> **Category:** Sprites

## SPHITINFO (1)

Gets information on collision detection results (Time of collision) If used before SPSET, an error will occur

### Format

```sb3
SPHITINFO OUT TM
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `TM` | - Variable that returns time of collision: real-type number from 0 to 1<br>- Position at collision detection + speed x collision time = collision X-Y coordinates |

### Examples

```sb3
SPHITINFO OUT TM
```

## SPHITINFO (2)

Gets information on collision detection results (Time of collision and coordinates) If used before SPSET, an error will occur

### Format

```sb3
SPHITINFO OUT TM,X1,Y1,X2,Y2
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `TM` | - Variable that returns time of collision: real-type number from 0 to 1<br>- Position at collision detection + speed x collision time = collision X-Y coordinates |
| `X1,Y1` | Variable that returns the X-Y coordinates of object 1 at the time of collision |
| `X2,Y2` | Variable that returns the X-Y coordinates of object 2 at the time of collision |

### Examples

```sb3
SPHITINFO OUT TM,X1,Y1,X2,Y2
```

## SPHITINFO (3)

Gets information on collision detection results (Time of collision, coordinates and speed) If used before SPSET, an error will occur

### Format

```sb3
SPHITINFO OUT TM,X1,Y1,VX1,VY1,X2,Y2,VX2,VY2
```

### Arguments

| Argument | Description |
| --- | --- |
| `None` |  |

### Return Values

| Return Value | Description |
| --- | --- |
| `Time of collision` | - Variable that returns time of collision: real-type number from 0 to 1<br>- Position at collision detection + speed x collision time = collision X-Y coordinates |
| `X1,Y1` | Variable that returns the X-Y coordinates of object 1 at the time of collision |
| `VX1,VY1` | Variable that returns the speed of object 1 at the time of collision |
| `X2,Y2` | Variable that returns the X-Y coordinates of object 2 at the time of collision |
| `VX2,VY2` | Variable that returns the speed of object 2 at the time of collision |

### Examples

```sb3
SPHITINFO OUT TM,X1,Y1,VX1,VY1,X2,Y2,VX2,VY2
```
