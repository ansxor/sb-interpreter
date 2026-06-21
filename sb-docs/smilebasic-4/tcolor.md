---
title: TCOLOR
slug: docs-sb4-tcolor
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-tcolor
content_id: 19545
created: 2020-05-06
scraped: 2026-06-21
---

# TCOLOR

Apply a color filter to the entire text screen. The color of all tiles is multiplied by the filter color.

/To change the default color of individual characters, see/ `COLOR`.

## Syntax

```sbsyntax
TCOLOR screenID%, color%
TCOLOR screenID% OUT color%
```

| Argument | Description |
| --- | --- |
| `screenID%` | ID of the target text screen |
| `color%` | The color used for the multiply filter |

## Examples

```sb4
'apply a red filter to text screen 0
TCOLOR 0,#C_RED
PRINT TCOLOR(0)  '> -65536
```

## See Also

- `COLOR`
- `RGB`
- `RGBF`
- `HSV`
- `HSVF`
