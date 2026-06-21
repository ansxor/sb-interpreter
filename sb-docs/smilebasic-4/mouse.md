---
title: MOUSE
slug: docs-sb4-mouse
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-mouse
content_id: 19523
created: 2021-12-02
scraped: 2026-06-21
---

# MOUSE

Set or get the status of the mouse, including the mouse cursor and wheel position. The mouse cursor is shared by both the USB mouse and the virtual mouse controlled by the right stick.

## Syntax

```sbfunction
MOUSE { coordFlag% } OUT cursorX%, cursorY% {, wheel% }
MOUSE cursorX%, cursorY% {, coordFlag% }
```

| Parameter | Output |
| --- | --- |
| `cursorX%` | The position of the mouse cursor. |
| `cursorY%` | |
| `wheel%` | The value of the mouse wheel. It cannot be set. |
| `coordFlag%` | If true, mouse cursor coordinates are given in Switch display pixels (1280x720). If false, coordinates are given in virtual BASIC screen pixels. Optional, default false. |

The position of the mouse cursor can be set in either Switch display or BASIC screen pixels. This can be used in programs that need to re-center the mouse cursor (first-person games) or software that allows control of the mouse in alternate ways.

SmileBASIC only recognizes a single wheel with a single axis on USB mice. This is typical of most mice, but it does mean that some mice won't work. There is also nothing corresponding to the wheel in the virtual mouse controls, thus the value read by the mouse wheel cannot be set.

## Examples

```sb4
VAR X%,Y%
LOOP
 VSYNC
 CLS
 MOUSE OUT X%,Y%
 PRINT X%,Y%
ENDLOOP
```
