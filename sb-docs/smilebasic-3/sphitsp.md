---
title: SPHITSP
slug: docs-sb3-sphitsp
system: SmileBASIC 3
type: command
category: Sprites
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# SPHITSP

> **Category:** Sprites

## SPHITSP (1)

Detects sprite collision

- SPCOL and SPCOLVEC should be called beforehand
- If used before SPSET, an error will occur

### Format

```sb3
Variable = SPHITSP( Management number [,First ID,Last ID] )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to detect collision with: 0-511 |
| `First ID,Last ID` | Range of sprites to detect (0-511) |

### Return Values

```
Management number of the colliding sprite (When no collision, -1)
```

### Examples

```sb3
H=SPHITSP(0)
```

## SPHITSP (2)

Detects sprite collision: collision between the specified sprites

- SPCOL and SPCOLVEC should be called beforehand
- If used before SPSET, an error will occur

### Format

```sb3
Variable = SPHITSP( Management number ,Opponent management number )
```

### Arguments

| Argument | Description |
| --- | --- |
| `Management number` | Management number of the sprite to detect collision with: 0-511 |
| `Opponent<br>management number` | Management number of the opponent sprite: 0-511 |

### Return Values

```
FALSE = No collision, TRUE = Collision
```

### Examples

```sb3
H=SPHITSP( 0,34 )
```
