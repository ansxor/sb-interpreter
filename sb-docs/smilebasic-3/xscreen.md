---
title: XSCREEN
slug: docs-sb3-xscreen
system: SmileBASIC 3
type: command
category: Screen control
source: InstructionList.pdf
forms: 1
scraped: 2026-06-21
---

# XSCREEN

> **Category:** Screen control

Sets a screen mode

- Screen modes 2 and 3 can also be used in DIRECT mode, but the Touch Screen will be switched to a keyboard after

execution is started

- 3D specification can be disabled in the Parental Control settings

## Format

```sb3
XSCREEN Screen mode [,Number of sprite assignments ,Number of BG assignments]
```

## Arguments

| Argument | Description |
| --- | --- |
| `Screen mode` | 0: Upper screen-3D, Touch Screen-Not used (Default)<br>1: Upper screen-2D, Touch Screen-Not used<br>2: Upper screen-3D, Touch Screen-Used (Keyboard displayed during INPUT)<br>3: Upper screen-2D, Touch Screen-Used (Keyboard displayed during INPUT)<br>4: Upper and Touch screens combined (Upper screen 2D; INPUT and DIRECT mode not allowed) |
| `Number of sprite<br>assignments` | - Number of sprites to assign to the upper screen: 0-512<br>- Touch Screen: 512 - number of SPs on the upper screen |
| `Number of BG<br>allocations` | - Number of BG layers to assign to the upper screen: 0-4<br>- Touch Screen: 4 - number of BG layers on the upper screen |

## Examples

```sb3
XSCREEN 2,128,4
```
