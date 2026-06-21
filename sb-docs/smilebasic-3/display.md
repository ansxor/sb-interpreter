---
title: DISPLAY
slug: docs-sb3-display
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 2
scraped: 2026-06-21
---

# DISPLAY

> **Category:** Screen control

## DISPLAY (1)

Selects the screen to manipulate (Upper or Touch)

- DISPLAY 1 can be specified when XSCREEN 2 or 3 is used
- This command cannot be directly executed in DIRECT mode.

### Format

```sb3
DISPLAY Screen ID
```

### Arguments

| Argument | Description |
| --- | --- |
| `Screen ID` | 0: Upper screen, 1: Touch Screen |

### Examples

```sb3
DISPLAY 0
```

## DISPLAY (2)

Gets the Screen ID that is currently being used

- DISPLAY 1 can be specified when XSCREEN 2 or 3 is used
- This command cannot be directly executed in DIRECT mode.

### Format

```sb3
Variable=DISPLAY()
```

### Return Values

```
Screen ID (0: Upper screen, 1: Touch Screen)
```

### Examples

```sb3
A=DISPLAY()
```
