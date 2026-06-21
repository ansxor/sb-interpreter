---
title: VISIBLE
slug: docs-sb3-visible
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# VISIBLE

> **Category:** Screen control

Switches screen display elements ON/OFF

## Format

```sb3
VISIBLE Console,Graphic,BG,sprite
```

## Arguments

| Argument | Description |
| --- | --- |
| `Console` | 0: Hide (#OFF), 1: Display (#ON) |
| `Graphic` | 0: Hide (#OFF), 1: Display (#ON) |
| `BG` | 0: Hide (#OFF), 1: Display (#ON) |
| `sprite` | 0: Hide (#OFF), 1: Display (#ON) |

## Examples

```sb3
VISIBLE 1,1,1,1
```
