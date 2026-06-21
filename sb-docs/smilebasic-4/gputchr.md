---
title: GPUTCHR
slug: docs-sb4-gputchr
system: SmileBASIC 4
type: command
source: https://smilebasicsource.com/forum/thread/docs-sb4-gputchr
content_id: 19529
created: 2020-05-07
scraped: 2026-06-21
---

# GPUTCHR

Draw text to a graphics page

## Syntax

```sbsyntax
GPUTCHR x%, y%, text [,font% [,color% [,drawingMethod%]]]
GPUTCHR x%, y%, text, font%, scaleX%, scaleY%, color% [,drawingMethod%]
```

| Argument | Description |
| --- | --- |
| `x%`, `y%` | Position to draw at (upper left corner of text) |
| `text` | The text to draw. Can be a string or a character code |
| `font%` | The font to use (8 or 16); defaults to 16 |
| `color%` | Text color; defaults to `GCOLOR()` |
| `scaleX%`, `scaleY%` | Text scale. (Must be an integer); defaults to 1,1 |
| `drawingMethod%` | See <some page we haven't written yet> |

## Examples

```sb4
GPUTCHR 10,10, "Hello!",16, 5,5, #C_RED 'Draw "Hello" in big red letters
```

## See Also

- GPUTCHRP
- GCOPY
- FONTINFO
