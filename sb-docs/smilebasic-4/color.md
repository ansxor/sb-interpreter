---
title: COLOR
slug: docs-sb4-color
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-color
content_id: 19546
created: 2020-05-06
scraped: 2026-06-21
---

# COLOR

Set/get the default color for characters on a text screen. This function affects the default colors for /new/ characters; existing characters are unchanged.

/To apply a color filter to a text screen, see/ `TCOLOR`.

## Syntax

```sbsyntax
COLOR { screenID%, } color%
COLOR { screenID% } OUT color%
```

| Argument | Description |
| --- | --- |
| `screenID%` | Target text screen; optional, default 4 (`#TCONSOLE`) |
| `color%` | Default color used by this text screen |

## Examples

```sb4
COLOR #C_RED
PRINT "This is red"
COLOR #C_GREEN
PRINT "This is green"
```

## See Also

- `TCOLOR`
- `RGB`
- `RGBF`
- `HSV`
- `HSVF`
