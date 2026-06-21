---
title: TPAGE
slug: docs-sb4-tpage
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-tpage
content_id: 19544
created: 2020-05-06
scraped: 2026-06-21
---

# TPAGE

Set/get the graphics page area used for user characters. User characters are displayed instead of the font for text characters `&hE800` (`#TUSRCHR`) through `&hF7FF` (`#TUSRCHR+4095`). By default, these use the background tile images in GRP4 (area starting at 1024,0).

## Syntax

```sbsyntax
TPAGE { screenID%, } graphicsPage%, areaU%, areaV%
TPAGE { screenID% } OUT graphicsPage%, areaU%, areaV%
```

| Argument | Description |
| --- | --- |
| `screenID%` | ID of target text screen; optional, default 4 (`#TCONSOLE`) |
| `graphicsPage%`| ID of graphics page to use as the character source |
| `areaU%`, `areaV%` | Upper-left coordinates of character reference area |

## Character Reference Area

The character reference area is the portion of the selected graphics page used for user characters. The `areaU%` and `areaV%` coordinates specify the upper-left corner of the reference area. User characters are arranged in a 64x64 grid in the reference area; the size of the area (in pixels) is based on the character size set in the text screen.

This table displays sizes of the reference area, based on the text screen's character type.

| Type | Size (pixels) |
| --- | --- |
| 8 | 512x512 |
| 16 | 1024x1024 |
| 24 | 1536x1536 |
| 32 | 2048x2048 |
| 40 | 2560x2560 |
| 48 | 3072x3072 |
| 56 | 3854x3854 |
| 64 | 4096x4096 |

For sizes greater than 32, the reference area is too big for one graphics page (graphics pages are 2048x2048). Regardless, the reference area always uses a 64x64 character grid even if the area is too large or positioned so that it goes off the graphics page. In this situation, the graphics page's `GSAMPLE` setting affects the appearance of the off-page characters.

## Examples

```sb4
'read TPAGE settings of the console screen
TPAGE OUT PAGE%,U%,V%
'set TPAGE settings of screen 2
'-use GRP 1
'-start reference area at 64,64
TPAGE 2,1,64,64
```

## See Also

- `TSCREEN`
