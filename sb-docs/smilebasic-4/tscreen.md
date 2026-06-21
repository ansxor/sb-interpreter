---
title: TSCREEN
slug: docs-sb4-tscreen
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-tscreen
content_id: 19543
created: 2020-05-05
scraped: 2026-06-21
---

# TSCREEN

Set/get the size and character type of a text screen.

## Syntax

```sbsyntax
TSCREEN charSize% {, displaySize% }
TSCREEN screenID%, charSize%, width%, height%
TSCREEN screenID%, charSize%, displaySize% {, width%, height% }
TSCREEN { screenID% } OUT charSize%, displaySize%, width%, height%
```

| Argument | Description |
| --- | --- |
|`screenId%`| 0 - 4; optional, defaults to 4 |
|`charSize%`| The size of the characters on the graphics page<br>can be _any multiple of 8_ between 8 and 64.<br>Text is only displayed for sizes 8 and 16 |
|`displaySize%`| The size that tiles are displayed at<br>Can be _any integer_ between 8 and 64. Tiles will be scaled if this is different than `charSize%` (similar to `TSCALE`) |
| `width%` | The size of the layer, in characters |
| `height%` | The size of the layer, in characters |

## Examples

```sb4
someone finish this aaaa
```
